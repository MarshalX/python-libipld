use std::io::{BufReader, BufWriter, Cursor, Read, Seek, Write};

use ::libipld::cbor::{cbor, cbor::MajorKind, decode, encode};
use ::libipld::cbor::error::{LengthOutOfRange, NumberOutOfRange, UnknownTag};
use ::libipld::cid::Cid;
use anyhow::Result;
use byteorder::{BigEndian, ByteOrder};
use futures::{executor, stream::StreamExt};
use iroh_car::{CarHeader, CarReader, Error};
use pyo3::{PyObject, Python};
use pyo3::conversion::ToPyObject;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyList};

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

fn decode_len(len: u64) -> Result<usize> {
    Ok(usize::try_from(len).map_err(|_| LengthOutOfRange::new::<usize>())?)
}

fn decode_dag_cbor_to_pyobject<R: Read + Seek>(py: Python, r: &mut R) -> Result<PyObject> {
    let major = decode::read_major(r)?;
    let py_object = match major.kind() {
        MajorKind::UnsignedInt => (decode::read_uint(r, major)? as i128).to_object(py),
        MajorKind::NegativeInt => (-1 - decode::read_uint(r, major)? as i128).to_object(py),
        MajorKind::ByteString => {
            let len = decode::read_uint(r, major)?;
            PyBytes::new(py, &decode::read_bytes(r, len)?).into()
        }
        MajorKind::TextString => {
            let len = decode::read_uint(r, major)?;
            decode::read_str(r, len)?.to_object(py)
        }
        MajorKind::Array => {
            let len = decode_len(decode::read_uint(r, major)?)?;
            // TODO (MarshalX): how to init list with capacity?
            let list = PyList::empty(py);
            for _ in 0..len {
                list.append(decode_dag_cbor_to_pyobject(py, r).unwrap()).unwrap();
            }
            list.into()
        }
        MajorKind::Map => {
            let len = decode_len(decode::read_uint(r, major)?)?;
            let dict = PyDict::new(py);
            for _ in 0..len {
                // FIXME (MarshalX): we should raise on duplicate keys?
                let key = decode_dag_cbor_to_pyobject(py, r).unwrap();
                let value = decode_dag_cbor_to_pyobject(py, r).unwrap();
                dict.set_item(key, value).unwrap();
            }
            dict.into()
        }
        MajorKind::Tag => {
            let value = decode::read_uint(r, major)?;
            if value != 42 {
                return Err(anyhow::anyhow!("non-42 tags are not supported"));
            }

            decode::read_link(r)?.to_string().to_object(py)
        }
        MajorKind::Other => match major {
            cbor::FALSE => false.to_object(py),
            cbor::TRUE => true.to_object(py),
            cbor::NULL => py.None(),
            cbor::F32 => (decode::read_f32(r)? as f64).to_object(py),
            cbor::F64 => decode::read_f64(r)?.to_object(py),
            _ => return Err(anyhow::anyhow!(format!("unsupported major type"))),
        },
    };
    Ok(py_object)
}

fn encode_dag_cbor_from_pyobject<W: Write>(py: Python, obj: &PyAny, w: &mut W) -> Result<()> {
    if obj.is_none() {
        encode::write_null(w)
    } else if let Ok(b) = obj.extract::<bool>() {
        let buf = if b { [cbor::TRUE.into()] } else { [cbor::FALSE.into()] };
        Ok(w.write_all(&buf)?)
    } else if let Ok(i) = obj.extract::<i128>() {
        if i < 0 {
            if -(i + 1) > u64::MAX as i128 {
                return Err(NumberOutOfRange::new::<i128>().into());
            }
            encode::write_u64(w, MajorKind::NegativeInt, -(i + 1) as u64)
        } else {
            if i > u64::MAX as i128 {
                return Err(NumberOutOfRange::new::<i128>().into());
            }
            encode::write_u64(w, MajorKind::UnsignedInt, i as u64)
        }
    } else if let Ok(f) = obj.extract::<f64>() {
        if !f.is_finite() {
            return Err(NumberOutOfRange::new::<f64>().into());
        }
        let mut buf = [0xfb, 0, 0, 0, 0, 0, 0, 0, 0];
        BigEndian::write_f64(&mut buf[1..], f);
        Ok(w.write_all(&buf)?)
    } else if let Ok(b) = obj.extract::<&[u8]>() {
        encode::write_u64(w, MajorKind::ByteString, b.len() as u64)?;
        Ok(w.write_all(b)?)
    } else if let Ok(s) = obj.extract::<String>() {
        // FIXME (MarshalX): it's not efficient to try to parse it as CID
        let cid = Cid::try_from(s.as_str());
        if let Ok(cid) = cid {
            encode::write_tag(w, 42)?;
            // FIXME (MarshalX): allocates
            let buf = cid.to_bytes();
            let len = buf.len();
            encode::write_u64(w, MajorKind::ByteString, len as u64 + 1)?;
            w.write_all(&[0])?;
            Ok(w.write_all(&buf[..len])?)
        } else {
            encode::write_u64(w, MajorKind::TextString, s.len() as u64)?;
            Ok(w.write_all(s.as_bytes())?)
        }
    } else if let Ok(l) = obj.downcast::<PyList>() {
        encode::write_u64(w, MajorKind::Array, l.len() as u64)?;
        for item in l.iter() {
            encode_dag_cbor_from_pyobject(py, item, w)?;
        }
        Ok(())
    } else if let Ok(d) = obj.downcast::<PyDict>() {
        encode::write_u64(w, MajorKind::Map, d.len() as u64)?;
        // FIXME (MarshalX): care about the order of keys?
        for (key, value) in d.iter() {
            encode_dag_cbor_from_pyobject(py, key, w)?;
            encode_dag_cbor_from_pyobject(py, value, w)?;
        }
        Ok(())
    } else {
        return Err(UnknownTag(0).into());
    }
}

#[pyfunction]
fn decode_dag_cbor_multi<'py>(py: Python<'py>, data: &[u8]) -> PyResult<&'py PyList> {
    let mut reader = BufReader::new(Cursor::new(data));
    let decoded_parts = PyList::empty(py);

    loop {
        let py_object = decode_dag_cbor_to_pyobject(py, &mut reader);
        if let Ok(py_object) = py_object {
            decoded_parts.append(py_object).unwrap();
        } else {
            break;
        }
    }

    Ok(decoded_parts)
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
            let py_object = decode_dag_cbor_to_pyobject(py, &mut BufReader::new(Cursor::new(bytes)));
            if let Ok(py_object) = py_object {
                let key = cid.to_string().to_object(py);
                parsed_blocks.set_item(key, py_object).unwrap();
            }
        }
    });

    Ok((header, parsed_blocks))
}

#[pyfunction]
fn decode_dag_cbor(py: Python, data: &[u8]) -> PyResult<PyObject> {
    let py_object = decode_dag_cbor_to_pyobject(py, &mut BufReader::new(Cursor::new(data)));
    if let Ok(py_object) = py_object {
        Ok(py_object)
    } else {
        Err(get_err("Failed to decode DAG-CBOR", py_object.unwrap_err().to_string()))
    }
}

#[pyfunction]
fn encode_dag_cbor<'py>(py: Python<'py>, data: &PyAny) -> PyResult<&'py PyBytes> {
    let mut buf = &mut BufWriter::new(Vec::new());
    if let Err(e) = encode_dag_cbor_from_pyobject(py, data, &mut buf) {
        return Err(get_err("Failed to encode DAG-CBOR", e.to_string()));
    }
    if let Err(e) = buf.flush() {
        return Err(get_err("Failed to flush buffer", e.to_string()));
    }
    Ok(PyBytes::new(py, &buf.get_ref()))
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
    m.add_function(wrap_pyfunction!(encode_dag_cbor, m)?)?;
    m.add_function(wrap_pyfunction!(decode_dag_cbor_multi, m)?)?;
    m.add_function(wrap_pyfunction!(decode_multibase, m)?)?;
    m.add_function(wrap_pyfunction!(encode_multibase, m)?)?;
    Ok(())
}
