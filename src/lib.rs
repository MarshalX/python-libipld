use std::io::{BufReader, BufWriter, Cursor, Read, Seek, Write};

use ::libipld::cbor::{cbor, cbor::MajorKind, decode, encode};
use ::libipld::cbor::error::{LengthOutOfRange, NumberOutOfRange, UnknownTag};
use ::libipld::cid::Cid;
use anyhow::Result;
use byteorder::{BigEndian, ByteOrder};
use futures::{executor, stream::StreamExt};
use iroh_car::{CarHeader, CarReader, Error as CarError};
use pyo3::{PyObject, Python};
use pyo3::conversion::ToPyObject;
use pyo3::prelude::*;
use pyo3::types::*;

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

fn map_key_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    /* The keys in every map must be sorted length-first by the byte representation of the string keys, where:
    - If two keys have different lengths, the shorter one sorts earlier;
    - If two keys have the same length, the one with the lower value in (byte-wise) lexical order sorts earlier.
     */
    if a.len() != b.len() {
        a.len().cmp(&b.len())
    } else {
        a.cmp(b)
    }
}

fn sort_map_keys(keys: &PySequence, len: usize) -> Vec<(&str, usize)> {
    // Returns key and index.
    let mut keys_str = Vec::with_capacity(len);
    for i in 0..len {
        let key: &PyString = keys.get_item(i).unwrap().downcast().unwrap();
        keys_str.push((key.to_str().unwrap(), i));
    }

    keys_str.sort_by(|a, b| {
        let (s1, _) = a;
        let (s2, _) = b;

        map_key_cmp(s1, s2)
    });

    keys_str
}

fn decode_dag_cbor_to_pyobject<R: Read + Seek>(py: Python, r: &mut R, deep: usize) -> Result<PyObject> {
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
                list.append(decode_dag_cbor_to_pyobject(py, r, deep + 1)?).unwrap();
            }
            list.into()
        }
        MajorKind::Map => {
            let len = decode_len(decode::read_uint(r, major)?)?;
            let dict = PyDict::new(py);

            let mut prev_key: Option<String> = None;
            for _ in 0..len {
                // DAG-CBOR keys are always strings
                let key_major = decode::read_major(r)?;
                if key_major.kind() != MajorKind::TextString {
                    return Err(anyhow::anyhow!("Map keys must be strings"));
                }

                let key_len = decode::read_uint(r, key_major)?;
                let key = decode::read_str(r, key_len)?;

                if let Some(prev_key) = prev_key {
                    if map_key_cmp(&prev_key, &key) == std::cmp::Ordering::Greater {
                        return Err(anyhow::anyhow!("Map keys must be sorted"));
                    }
                }

                let key_py = key.to_object(py);
                prev_key = Some(key);
                if dict.get_item(&key_py)?.is_some() {
                    return Err(anyhow::anyhow!("Duplicate keys are not allowed"));
                }

                let value = decode_dag_cbor_to_pyobject(py, r, deep + 1)?;
                dict.set_item(key_py, value).unwrap();
            }
            dict.into()
        }
        MajorKind::Tag => {
            let value = decode::read_uint(r, major)?;
            if value != 42 {
                return Err(anyhow::anyhow!("Non-42 tags are not supported"));
            }

            decode::read_link(r)?.to_string().to_object(py)
        }
        MajorKind::Other => match major {
            cbor::FALSE => false.to_object(py),
            cbor::TRUE => true.to_object(py),
            cbor::NULL => py.None(),
            cbor::F32 => (decode::read_f32(r)? as f64).to_object(py),
            cbor::F64 => decode::read_f64(r)?.to_object(py),
            _ => return Err(anyhow::anyhow!(format!("Unsupported major type"))),
        },
    };
    Ok(py_object)
}

fn encode_dag_cbor_from_pyobject<'py, W: Write>(py: Python<'py>, obj: &'py PyAny, w: &mut W) -> Result<()> {
    /* Order is important for performance!

    Fast checks go first:
    - None
    - bool
    - int
    - list
    - dict
    - str
    Then slow checks:
    - bytes
    - float
     */

    if obj.is_none() {
        encode::write_null(w)?;

        Ok(())
    } else if obj.is_instance_of::<PyBool>() {
        let buf = if obj.is_true()? { [cbor::TRUE.into()] } else { [cbor::FALSE.into()] };
        w.write_all(&buf)?;

        Ok(())
    } else if obj.is_instance_of::<PyInt>() {
        let i: i128 = obj.extract()?;

        if i < 0 {
            if -(i + 1) > u64::MAX as i128 {
                return Err(NumberOutOfRange::new::<i128>().into());
            }
            encode::write_u64(w, MajorKind::NegativeInt, -(i + 1) as u64)?;
        } else {
            if i > u64::MAX as i128 {
                return Err(NumberOutOfRange::new::<i128>().into());
            }
            encode::write_u64(w, MajorKind::UnsignedInt, i as u64)?;
        }

        Ok(())
    } else if obj.is_instance_of::<PyList>() {
        let seq: &PySequence = obj.downcast().unwrap();
        let len = obj.len()?;

        encode::write_u64(w, MajorKind::Array, len as u64)?;

        for i in 0..len {
            encode_dag_cbor_from_pyobject(py, seq.get_item(i)?, w)?;
        }

        Ok(())
    } else if obj.is_instance_of::<PyDict>() {
        let map: &PyMapping = obj.downcast().unwrap();
        let keys = map.keys()?;
        let values = map.values()?;
        let len = map.len()?;

        encode::write_u64(w, MajorKind::Map, len as u64)?;

        let sorted_keys = sort_map_keys(&keys, len);
        for (key, i) in sorted_keys {
            let key_buf = key.as_bytes();
            encode::write_u64(w, MajorKind::TextString, key_buf.len() as u64)?;
            w.write_all(key_buf)?;

            encode_dag_cbor_from_pyobject(py, values.get_item(i)?, w)?;
        }

        Ok(())
    } else if obj.is_instance_of::<PyFloat>() {
        let f: &PyFloat = obj.downcast().unwrap();
        let v = f.value();

        if !v.is_finite() {
            return Err(NumberOutOfRange::new::<f64>().into());
        }

        let mut buf = [0xfb, 0, 0, 0, 0, 0, 0, 0, 0];
        BigEndian::write_f64(&mut buf[1..], v);
        w.write_all(&buf)?;

        Ok(())
    } else if obj.is_instance_of::<PyBytes>() {
        let b: &PyBytes = obj.downcast().unwrap();
        let l: u64 = b.len()? as u64;

        encode::write_u64(w, MajorKind::ByteString, l)?;
        w.write_all(b.as_bytes())?;

        Ok(())
    } else if obj.is_instance_of::<PyString>() {
        let s: &PyString = obj.downcast().unwrap();

        // FIXME (MarshalX): it's not efficient to try to parse it as CID
        let cid = Cid::try_from(s.to_str()?);
        if let Ok(cid) = cid {
            // FIXME (MarshalX): allocates
            let buf = cid.to_bytes();
            let len = buf.len();

            encode::write_tag(w, 42)?;
            encode::write_u64(w, MajorKind::ByteString, len as u64 + 1)?;
            w.write_all(&[0])?;
            w.write_all(&buf[..len])?;

            Ok(())
        } else {
            let buf = s.to_str()?.as_bytes();

            encode::write_u64(w, MajorKind::TextString, buf.len() as u64)?;
            w.write_all(buf)?;

            Ok(())
        }
    } else {
        return Err(UnknownTag(0).into());
    }
}

#[pyfunction]
fn decode_dag_cbor_multi<'py>(py: Python<'py>, data: &[u8]) -> PyResult<&'py PyList> {
    let mut reader = BufReader::new(Cursor::new(data));
    let decoded_parts = PyList::empty(py);

    loop {
        let py_object = decode_dag_cbor_to_pyobject(py, &mut reader, 0);
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

    let blocks: Vec<Result<(Cid, Vec<u8>), CarError>> = executor::block_on(car.stream().collect());
    blocks.into_iter().for_each(|block| {
        if let Ok((cid, bytes)) = block {
            let py_object = decode_dag_cbor_to_pyobject(py, &mut BufReader::new(Cursor::new(bytes)), 0);
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
    let py_object = decode_dag_cbor_to_pyobject(py, &mut BufReader::new(Cursor::new(data)), 0);
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
fn encode_multibase(code: char, data: &PyAny) -> PyResult<String> {
    let data_bytes: &[u8];
    if data.is_instance_of::<PyBytes>() {
        let b: &PyBytes = data.downcast().unwrap();
        data_bytes = b.as_bytes();
    } else if data.is_instance_of::<PyByteArray>() {
        let b: &PyByteArray = data.downcast().unwrap();
        data_bytes = unsafe { b.as_bytes() };
    } else if data.is_instance_of::<PyString>() {
        let s: &PyString = data.downcast().unwrap();
        data_bytes = s.to_str()?.as_bytes();
    } else {
        return Err(get_err("Failed to encode multibase", "Unsupported data type".to_string()));
    }

    let base = multibase::Base::from_code(code);
    if let Ok(base) = base {
        Ok(multibase::encode(base, data_bytes))
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
