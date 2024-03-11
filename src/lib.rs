use std::io::{BufReader, BufWriter, Cursor, Read, Seek, Write};

use ::libipld::cbor::{cbor, cbor::MajorKind, decode, encode};
use ::libipld::cbor::error::{LengthOutOfRange, NumberOutOfRange, UnknownTag};
use ::libipld::cid::Cid;
use ::libipld::cid::Version;
use ::libipld::cid::Result as CidResult;
use ::libipld::cid::Error as CidError;
use anyhow::Result;
use byteorder::{BigEndian, ByteOrder};
use pyo3::{PyObject, Python};
use pyo3::conversion::ToPyObject;
use pyo3::prelude::*;
use pyo3::types::*;
use pyo3::pybacked::PyBackedStr;
use multihash::{Multihash};

fn car_header_to_pydict<'py>(py: Python<'py>, header: &CarHeader) -> Bound<'py, PyDict> {
    let dict_obj = PyDict::new_bound(py);

    dict_obj.set_item("version", header.version()).unwrap();

    let roots = PyList::empty_bound(py);
    header.roots().iter().for_each(|cid| {
        let cid_obj = cid.to_string().to_object(py);
        roots.append(cid_obj).unwrap();
    });

    dict_obj.set_item("roots", roots).unwrap();

    dict_obj
}

fn cid_hash_to_pydict<'py>(py: Python<'py>, cid: &Cid) -> Bound<'py, PyDict> {
    let hash = cid.hash();
    let dict_obj = PyDict::new_bound(py);

    dict_obj.set_item("code", hash.code()).unwrap();
    dict_obj.set_item("size", hash.size()).unwrap();
    dict_obj.set_item("digest", PyBytes::new_bound(py, &hash.digest())).unwrap();

    dict_obj
}

fn cid_to_pydict<'py>(py: Python<'py>, cid: &Cid) -> Bound<'py, PyDict> {
    let dict_obj = PyDict::new_bound(py);

    dict_obj.set_item("version", cid.version() as u64).unwrap();
    dict_obj.set_item("codec", cid.codec()).unwrap();
    dict_obj.set_item("hash", cid_hash_to_pydict(py, cid)).unwrap();

    dict_obj
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

fn sort_map_keys(keys: &Bound<PySequence>, len: usize) -> Vec<(PyBackedStr, usize)> {
    // Returns key and index.
    let mut keys_str = Vec::with_capacity(len);
    for i in 0..len {
        let item = keys.get_item(i).unwrap();
        let key = item.downcast::<PyString>().unwrap().to_owned();
        let backed_str = PyBackedStr::try_from(key).unwrap();
        keys_str.push((backed_str, i));
    }

    keys_str.sort_by(|a, b| {  // sort_unstable_by performs bad
        let (s1, _) = a;
        let (s2, _) = b;

        map_key_cmp(s1, s2)
    });

    keys_str
}

fn decode_dag_cbor_to_pyobject<R: Read + Seek>(py: Python, r: &mut R, deep: usize) -> Result<PyObject> {
    let major = decode::read_major(r)?;
    let py_object = match major.kind() {
        MajorKind::UnsignedInt => (decode::read_uint(r, major)?).to_object(py),
        MajorKind::NegativeInt => (-1 - decode::read_uint(r, major)? as i64).to_object(py),
        MajorKind::ByteString => {
            let len = decode::read_uint(r, major)?;
            PyBytes::new_bound(py, &decode::read_bytes(r, len)?).to_object(py)
        }
        MajorKind::TextString => {
            let len = decode::read_uint(r, major)?;
            PyString::new_bound(py, &decode::read_str(r, len)?).to_object(py)
        }
        MajorKind::Array => {
            let len = decode_len(decode::read_uint(r, major)?)?;
            let list = PyList::empty_bound(py);

            for _ in 0..len {
                list.append(decode_dag_cbor_to_pyobject(py, r, deep + 1)?)?;
            }

            list.to_object(py)
        }
        MajorKind::Map => {
            let len = decode_len(decode::read_uint(r, major)?)?;
            let dict = PyDict::new_bound(py);

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

            dict.to_object(py)
        }
        MajorKind::Tag => {
            let value = decode::read_uint(r, major)?;
            if value != 42 {
                return Err(anyhow::anyhow!("Non-42 tags are not supported"));
            }

            let cid = decode::read_link(r)?.to_string();
            PyString::new_bound(py, &cid).to_object(py)
        }
        MajorKind::Other => match major {
            cbor::FALSE => false.to_object(py),
            cbor::TRUE => true.to_object(py),
            cbor::NULL => py.None(),
            cbor::F32 => decode::read_f32(r)?.to_object(py),
            cbor::F64 => decode::read_f64(r)?.to_object(py),
            _ => return Err(anyhow::anyhow!(format!("Unsupported major type"))),
        },
    };
    Ok(py_object)
}

fn encode_dag_cbor_from_pyobject<'py, W: Write>(py: Python<'py>, obj: &Bound<'py, PyAny>, w: &mut W) -> Result<()> {
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
        let buf = if obj.is_truthy()? { [cbor::TRUE.into()] } else { [cbor::FALSE.into()] };
        w.write_all(&buf)?;

        Ok(())
    } else if obj.is_instance_of::<PyInt>() {
        let i: i64 = obj.extract()?;

        if i.is_negative() {
            encode::write_u64(w, MajorKind::NegativeInt, -(i + 1) as u64)?
        } else {
            encode::write_u64(w, MajorKind::UnsignedInt, i as u64)?
        }

        Ok(())
    } else if obj.is_instance_of::<PyList>() {
        let seq = obj.downcast::<PySequence>().unwrap();
        let len = obj.len()?;

        encode::write_u64(w, MajorKind::Array, len as u64)?;

        for i in 0..len {
            encode_dag_cbor_from_pyobject(py, &seq.get_item(i)?, w)?;
        }

        Ok(())
    } else if obj.is_instance_of::<PyDict>() {
        let map = obj.downcast::<PyMapping>().unwrap();
        let len = map.len()?;
        let keys = sort_map_keys(&map.keys()?, len);
        let values = map.values()?;

        encode::write_u64(w, MajorKind::Map, len as u64)?;

        for (key, i) in keys {
            let key_buf = key.as_bytes();
            encode::write_u64(w, MajorKind::TextString, key_buf.len() as u64)?;
            w.write_all(key_buf)?;

            encode_dag_cbor_from_pyobject(py, &values.get_item(i)?, w)?;
        }

        Ok(())
    } else if obj.is_instance_of::<PyFloat>() {
        let f = obj.downcast::<PyFloat>().unwrap();
        let v = f.value();

        if !v.is_finite() {
            return Err(NumberOutOfRange::new::<f64>().into());
        }

        let mut buf = [0xfb, 0, 0, 0, 0, 0, 0, 0, 0];
        BigEndian::write_f64(&mut buf[1..], v);
        w.write_all(&buf)?;

        Ok(())
    } else if obj.is_instance_of::<PyBytes>() {
        let b = obj.downcast::<PyBytes>().unwrap();
        let l: u64 = b.len()? as u64;

        encode::write_u64(w, MajorKind::ByteString, l)?;
        w.write_all(b.as_bytes())?;

        Ok(())
    } else if obj.is_instance_of::<PyString>() {
        let s = obj.downcast::<PyString>().unwrap();

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
fn decode_dag_cbor_multi<'py>(py: Python<'py>, data: &[u8]) -> PyResult<Bound<'py, PyList>> {
    let mut reader = BufReader::new(Cursor::new(data));
    let decoded_parts = PyList::empty_bound(py);

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

#[inline]
fn read_u64_leb128<R: Read>(r: &mut R) -> Result<u64> {
    let mut result = 0;
    let mut shift = 0;

    loop {
        let mut buf = [0];
        if let Err(_) = r.read_exact(&mut buf) {
            return Err(anyhow::anyhow!("Unexpected EOF while reading ULEB128 number."));
        }

        let byte = buf[0] as u64;
        if (byte & 0x80) == 0 {
            result |= (byte) << shift;
            return Ok(result);
        } else {
            result |= (byte & 0x7F) << shift;
        }

        shift += 7;
    }
}

fn read_cid_from_bytes<R: Read>(r: &mut R) -> CidResult<Cid> {
    let version = read_u64_leb128(r).unwrap();
    let codec = read_u64_leb128(r).unwrap();

    if [version, codec] == [0x12, 0x20] {
        let mut digest = [0u8; 32];
        r.read_exact(&mut digest)?;
        let mh = Multihash::wrap(version, &digest).expect("Digest is always 32 bytes.");
        return Cid::new_v0(mh);
    }

    let version = Version::try_from(version)?;
    match version {
        Version::V0 => Err(CidError::InvalidCidVersion),
        Version::V1 => {
            let mh = Multihash::read(r)?;
            Cid::new(version, codec, mh)
        }
    }
}

#[pyfunction]
pub fn decode_car<'py>(py: Python<'py>, data: &[u8]) -> PyResult<(Bound<'py, PyDict>, Bound<'py, PyDict>)> {
    let buf = &mut BufReader::new(Cursor::new(data));

    let _ = read_u64_leb128(buf);
    let header = decode_dag_cbor_to_pyobject(py, buf, 0).unwrap();
    let parsed_blocks = PyDict::new_bound(py);

    loop {
        if let Err(_) = read_u64_leb128(buf) {
            break;
        }

        let cid = read_cid_from_bytes(buf);
        if let Err(_) = cid {
            break;
        }

        let block = decode_dag_cbor_to_pyobject(py, buf, 0);
        if let Ok(block) = block {
            parsed_blocks.set_item(cid.unwrap().to_string().to_object(py), block).unwrap();
        } else {
            break;
        }
    }

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
fn encode_dag_cbor<'py>(py: Python<'py>, data: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyBytes>> {
    let mut buf = &mut BufWriter::new(Vec::new());
    if let Err(e) = encode_dag_cbor_from_pyobject(py, data, &mut buf) {
        return Err(get_err("Failed to encode DAG-CBOR", e.to_string()));
    }
    if let Err(e) = buf.flush() {
        return Err(get_err("Failed to flush buffer", e.to_string()));
    }
    Ok(PyBytes::new_bound(py, &buf.get_ref()))
}

#[pyfunction]
fn decode_cid<'py>(py: Python<'py>, data: &str) -> PyResult<Bound<'py, PyDict>> {
    let cid = Cid::try_from(data);
    if let Ok(cid) = cid {
        Ok(cid_to_pydict(py, &cid))
    } else {
        Err(get_err("Failed to decode CID", cid.unwrap_err().to_string()))
    }
}

#[pyfunction]
fn decode_multibase<'py>(py: Python<'py>, data: &str) -> PyResult<(char, Bound<'py, PyBytes>)> {
    let base = multibase::decode(data);
    if let Ok((base, data)) = base {
        Ok((base.code(), PyBytes::new_bound(py, &data)))
    } else {
        Err(get_err("Failed to decode multibase", base.unwrap_err().to_string()))
    }
}

#[pyfunction]
fn encode_multibase(code: char, data: &Bound<PyAny>) -> PyResult<String> {
    let data_bytes: &[u8];
    if data.is_instance_of::<PyBytes>() {
        let b = data.downcast::<PyBytes>().unwrap();
        data_bytes = b.as_bytes();
    } else if data.is_instance_of::<PyByteArray>() {
        let ba = data.downcast::<PyByteArray>().unwrap();
        data_bytes = unsafe { ba.as_bytes() };
    } else if data.is_instance_of::<PyString>() {
        let s = data.downcast::<PyString>().unwrap();
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
fn libipld(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(decode_cid, m)?)?;
    m.add_function(wrap_pyfunction!(decode_car, m)?)?;
    m.add_function(wrap_pyfunction!(decode_dag_cbor, m)?)?;
    m.add_function(wrap_pyfunction!(encode_dag_cbor, m)?)?;
    m.add_function(wrap_pyfunction!(decode_dag_cbor_multi, m)?)?;
    m.add_function(wrap_pyfunction!(decode_multibase, m)?)?;
    m.add_function(wrap_pyfunction!(encode_multibase, m)?)?;
    Ok(())
}
