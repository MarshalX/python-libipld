use std::io::{BufReader, Cursor, Read, Seek};

use ::libipld::{cid::Cid, Ipld};
use ::libipld::cbor::{cbor::MajorKind, DagCborCodec, decode};
use ::libipld::prelude::Codec;
use anyhow::Result;
use futures::{executor, stream::StreamExt};
use iroh_car::{CarHeader, CarReader, Error};
use pyo3::{PyObject, Python};
use pyo3::conversion::ToPyObject;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyList};

fn ipld_to_pyobject(py: Python<'_>, ipld: &Ipld) -> PyObject {
    match ipld {
        Ipld::Null => py.None(),
        Ipld::Bool(b) => b.to_object(py),
        Ipld::Integer(i) => i.to_object(py),
        Ipld::Float(f) => f.to_object(py),
        Ipld::String(s) => s.to_object(py),
        Ipld::Bytes(b) => PyBytes::new(py, b).into(),
        Ipld::Link(cid) => cid.to_string().to_object(py),
        Ipld::List(l) => {
            let list_obj = PyList::empty(py);
            l.iter().for_each(|item| {
                let item_obj = ipld_to_pyobject(py, item);
                list_obj.append(item_obj).unwrap();
            });
            list_obj.into()
        }
        Ipld::Map(m) => {
            let dict_obj = PyDict::new(py);
            m.iter().for_each(|(key, value)| {
                let key_obj = key.to_object(py);
                let value_obj = ipld_to_pyobject(py, value);
                dict_obj.set_item(key_obj, value_obj).unwrap();
            });
            dict_obj.into()
        }
    }
}

fn car_header_to_pydict<'py>(py: Python<'py>, header: &CarHeader) -> &'py PyDict {
    let dict_obj = PyDict::new(py);

    dict_obj.set_item("version", header.version()).unwrap();

    let roots = PyList::empty(py);
    header.roots().iter().for_each(|cid| {
        let cid_obj = cid.to_string().to_object(py);
        roots.append(cid_obj).unwrap();
    });

    dict_obj.set_item("roots", roots).unwrap();

    dict_obj.into()
}

fn cid_hash_to_pydict<'py>(py: Python<'py>, cid: &Cid) -> &'py PyDict {
    let hash = cid.hash();
    let dict_obj = PyDict::new(py);

    dict_obj.set_item("code", hash.code()).unwrap();
    dict_obj.set_item("size", hash.size()).unwrap();
    dict_obj.set_item("digest", PyBytes::new(py, &hash.digest())).unwrap();

    dict_obj.into()
}

fn cid_to_pydict<'py>(py: Python<'py>, cid: &Cid) -> &'py PyDict {
    let dict_obj = PyDict::new(py);

    dict_obj.set_item("version", cid.version() as u64).unwrap();
    dict_obj.set_item("codec", cid.codec()).unwrap();
    dict_obj.set_item("hash", cid_hash_to_pydict(py, cid)).unwrap();

    dict_obj.into()
}

fn parse_dag_cbor_object<R: Read + Seek>(r: &mut R) -> Result<Ipld> {
    let major = decode::read_major(r)?;
    Ok(match major.kind() {
        MajorKind::UnsignedInt | MajorKind::NegativeInt => Ipld::Integer(major.info() as i128),
        MajorKind::ByteString => Ipld::Bytes(decode::read_bytes(r, major.info() as u64)?),
        MajorKind::TextString => Ipld::String(decode::read_str(r, major.info() as u64)?),
        MajorKind::Array => Ipld::List(decode::read_list(r, major.info() as u64)?),
        MajorKind::Map => Ipld::Map(decode::read_map(r, major.info() as u64)?),
        MajorKind::Tag => {
            if major.info() != 42 {
                return Err(anyhow::anyhow!("non-42 tags are not supported"));
            }

            Ipld::Link(decode::read_link(r)?)
        }
        MajorKind::Other => Ipld::Null,
    })
}

#[pyfunction]
fn decode_dag_cbor_multi(py: Python, data: &[u8]) -> PyResult<Vec<PyObject>> {
    let mut reader = BufReader::new(Cursor::new(data));
    let mut parts = Vec::new();

    loop {
        let ipld = parse_dag_cbor_object(&mut reader);
        if let Ok(cbor) = ipld {
            parts.push(ipld_to_pyobject(py, &cbor));
        } else {
            break;
        }
    }

    Ok(parts)
}

#[pyfunction]
pub fn decode_car<'py>(py: Python<'py>, data: &[u8]) -> PyResult<(&'py PyDict, &'py PyDict)> {
    let car_response = executor::block_on(CarReader::new(data));
    if let Err(e) = car_response {
        return Err(get_err("Failed to decode CAR", e.to_string()));
    }

    let car = car_response.unwrap();

    let header = car_header_to_pydict(py, car.header());
    let parsed_blocks = PyDict::new(py);

    let blocks: Vec<Result<(Cid, Vec<u8>), Error>> = executor::block_on(car.stream().collect());
    blocks.into_iter().for_each(|block| {
        if let Ok((cid, bytes)) = block {
            let ipld = DagCborCodec.decode(&bytes);
            if let Ok(ipld) = ipld {
                let key = cid.to_string().to_object(py);
                let value = ipld_to_pyobject(py, &ipld);
                parsed_blocks.set_item(key, value).unwrap();
            }
        }
    });

    Ok((header, parsed_blocks))
}

#[pyfunction]
fn decode_dag_cbor(py: Python, data: &[u8]) -> PyResult<PyObject> {
    let ipld = DagCborCodec.decode(data);
    if let Ok(ipld) = ipld {
        Ok(ipld_to_pyobject(py, &ipld))
    } else {
        Err(get_err("Failed to decode DAG-CBOR", ipld.unwrap_err().to_string()))
    }
}

#[pyfunction]
fn decode_cid(py: Python, data: String) -> PyResult<&PyDict> {
    let cid = Cid::try_from(data.as_str());
    if let Ok(cid) = cid {
        Ok(cid_to_pydict(py, &cid))
    } else {
        Err(get_err("Failed to decode CID", cid.unwrap_err().to_string()))
    }
}

#[pyfunction]
fn decode_multibase(py: Python, data: String) -> PyResult<(char, PyObject)> {
    let base = multibase::decode(data);
    if let Ok((base, data)) = base {
        Ok((base.code(), PyBytes::new(py, &data).into()))
    } else {
        Err(get_err("Failed to decode multibase", base.unwrap_err().to_string()))
    }
}

#[pyfunction]
fn encode_multibase(code: char, data: &[u8]) -> PyResult<String> {
    let base = multibase::Base::from_code(code);
    if let Ok(base) = base {
        Ok(multibase::encode(base, data))
    } else {
        Err(get_err("Failed to encode multibase", base.unwrap_err().to_string()))
    }
}

fn get_err(msg: &str, err: String) -> PyErr {
    PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}. {}", msg, err))
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
