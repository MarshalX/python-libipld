use std::borrow::Cow;
use std::collections::{HashMap};
use std::io::{BufReader, Cursor, Read, Seek};
use pyo3::prelude::*;
use pyo3::conversion::ToPyObject;
use pyo3::{PyObject, Python};
use pyo3::types::{PyBytes};
use anyhow::Result;
use iroh_car::{CarHeader, CarReader};
use futures::{executor, stream::StreamExt};
use ::libipld::cbor::cbor::MajorKind;
use ::libipld::cbor::decode;
use ::libipld::{cid::Cid, Ipld};


#[derive(Clone, PartialEq)]
pub enum HashMapItem {
    Null,
    Bool(bool),
    Integer(i128),
    Float(f64),
    String(String),
    List(Vec<HashMapItem>),
    Map(HashMap<String, HashMapItem>),
    Bytes(Cow<'static, [u8]>),
}

impl HashMapItem {
    fn value(&self) -> PyObject {
        Python::with_gil(|py| match self {
            Self::Null => py.None(),
            Self::Bool(b) => b.to_object(py),
            Self::String(s) => s.to_object(py),
            Self::Integer(i) => i.to_object(py),
            Self::Float(f) => f.to_object(py),
            Self::List(l) => l.to_object(py),
            Self::Map(m) => m.to_object(py),
            Self::Bytes(b) => b.to_object(py),
        })
    }
}

impl ToPyObject for HashMapItem {
    fn to_object(&self, _: Python<'_>) -> PyObject {
        self.value().into()
    }
}

impl IntoPy<Py<PyAny>> for HashMapItem {
    fn into_py(self, _: Python<'_>) -> Py<PyAny> {
        self.value().into()
    }
}


fn ipld_to_hashmap(x: Ipld) -> HashMapItem {
    match x {
        Ipld::Null => HashMapItem::Null,
        Ipld::Bool(b) => HashMapItem::Bool(b),
        Ipld::Integer(i) => HashMapItem::Integer(i),
        Ipld::Float(f) => HashMapItem::Float(f),
        Ipld::String(s) => HashMapItem::String(s),
        Ipld::Bytes(b) => HashMapItem::Bytes(Cow::Owned(b)),
        Ipld::List(l) => HashMapItem::List(l.into_iter().map(ipld_to_hashmap).collect()),
        Ipld::Map(m) => HashMapItem::Map(
            m.into_iter()
                .map(|(k, v)| (k, ipld_to_hashmap(v)))
                .collect(),
        ),
        Ipld::Link(cid) => HashMapItem::String(cid.to_string()),
    }
}

fn car_header_to_hashmap(header: &CarHeader) -> HashMapItem {
    HashMapItem::Map(
        vec![
            ("version".to_string(), HashMapItem::Integer(header.version() as i128)),
            (
                "roots".to_string(),
                HashMapItem::List(
                    header
                        .roots()
                        .iter()
                        .map(|cid| HashMapItem::String(cid.to_string()))
                        .collect(),
                ),
            ),
        ]
            .into_iter()
            .collect(),
    )
}

fn _cid_hash_to_hashmap(cid: &Cid) -> HashMapItem {
    let hash = cid.hash();
    HashMapItem::Map(
        vec![
            ("code".to_string(), HashMapItem::Integer(hash.code() as i128)),
            ("size".to_string(), HashMapItem::Integer(hash.size() as i128)),
            ("digest".to_string(), HashMapItem::Bytes(Cow::Owned(hash.digest().to_vec()))),
        ]
            .into_iter()
            .collect(),
    )
}

fn cid_to_hashmap(cid: &Cid) -> HashMapItem {
    HashMapItem::Map(
        vec![
            ("version".to_string(), HashMapItem::Integer(cid.version() as i128)),
            ("codec".to_string(), HashMapItem::Integer(cid.codec() as i128)),
            ("hash".to_string(), _cid_hash_to_hashmap(cid)),
        ]
            .into_iter()
            .collect(),
    )
}

fn parse_dag_cbor_object<R: Read + Seek>(mut reader: &mut BufReader<R>) -> Result<Ipld> {
    let major = decode::read_major(&mut reader)?;
    Ok(match major.kind() {
        MajorKind::UnsignedInt | MajorKind::NegativeInt => Ipld::Integer(major.info() as i128),
        MajorKind::ByteString => Ipld::Bytes(decode::read_bytes(&mut reader, major.info() as u64)?),
        MajorKind::TextString => Ipld::String(decode::read_str(&mut reader, major.info() as u64)?),
        MajorKind::Array => Ipld::List(decode::read_list(&mut reader, major.info() as u64)?),
        MajorKind::Map => Ipld::Map(decode::read_map(&mut reader, major.info() as u64)?),
        MajorKind::Tag => {
            if major.info() != 42 {
                return Err(anyhow::anyhow!("non-42 tags are not supported"));
            }

            parse_dag_cbor_object(reader)?
        }
        MajorKind::Other => Ipld::Null,
    })
}

#[pyfunction]
fn decode_dag_cbor_multi(data: Vec<u8>) -> PyResult<Vec<HashMapItem>> {
    let mut reader = BufReader::new(Cursor::new(data));

    let mut parts = Vec::new();
    loop {
        let cbor = parse_dag_cbor_object(&mut reader);
        if let Ok(cbor) = cbor {
            parts.push(_ipld_to_python(cbor));
        } else {
            break;
        }
    }
    Ok(parts)
}

fn _decode_dag_cbor(data: Vec<u8>) -> Result<Ipld> {
    let mut reader = BufReader::new(Cursor::new(data));
    parse_dag_cbor_object(&mut reader)
}

fn _ipld_to_python(ipld: Ipld) -> HashMapItem {
    ipld_to_hashmap(ipld.clone())
}

#[pyfunction]
fn decode_car(data: Vec<u8>) -> (HashMapItem, HashMap<String, HashMapItem>) {
    let car = executor::block_on(CarReader::new(data.as_slice())).unwrap();
    let header = car_header_to_hashmap(car.header());
    let blocks = executor::block_on(car
        .stream()
        .filter_map(|block| async {
            if let Ok((cid, bytes)) = block {
                let mut reader = BufReader::new(Cursor::new(bytes));

                let ipld = parse_dag_cbor_object(&mut reader);
                if let Ok(ipld) = ipld {
                    Some((cid.to_string(), ipld))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect::<HashMap<String, Ipld>>());

    let mut decoded_blocks = HashMap::new();
    for (cid, ipld) in &blocks {
        decoded_blocks.insert(cid.to_string(), _ipld_to_python(ipld.clone()));
    }

    (header, decoded_blocks)
}

#[pyfunction]
fn decode_dag_cbor(data: Vec<u8>) -> PyResult<HashMapItem> {
    Ok(_ipld_to_python(_decode_dag_cbor(data)?))
}

#[pyfunction]
fn decode_cid(data: String) -> PyResult<HashMapItem> {
    let cid = Cid::try_from(data.as_str()).unwrap();
    Ok(cid_to_hashmap(&cid))
}

#[pyfunction]
fn decode_multibase(py: Python, data: String) -> (char, PyObject) {
    let (base, data) = multibase::decode(data).unwrap();
    (base.code(), PyBytes::new(py, &data).into())
}

#[pyfunction]
fn encode_multibase(code: char, data: Vec<u8>) -> String {
    let base = multibase::Base::from_code(code).unwrap();
    let encoded = multibase::encode(base, data);
    encoded
}

#[pymodule]
fn libipld(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(decode_cid, m)?)?;
    m.add_function(wrap_pyfunction!(decode_car, m)?)?;
    m.add_function(wrap_pyfunction!(decode_dag_cbor, m)?)?;
    m.add_function(wrap_pyfunction!(decode_dag_cbor_multi, m)?)?;
    m.add_function(wrap_pyfunction!(decode_multibase, m)?)?;
    m.add_function(wrap_pyfunction!(encode_multibase, m)?)?;
    Ok(())
}
