use std::io::{BufReader, BufWriter, Cursor, Read, Seek, Write};
use std::os::raw::c_char;

use ::libipld::cbor::error::{LengthOutOfRange, NumberOutOfRange, UnknownTag};
use ::libipld::cbor::{cbor, cbor::MajorKind, decode, encode};
use ::libipld::cid::{Cid, Error as CidError, Result as CidResult, Version};
use anyhow::{anyhow, Result};
use byteorder::{BigEndian, ByteOrder};
use multihash::Multihash;
use pyo3::{ffi, prelude::*, types::*, BoundObject, PyObject, Python};
use pyo3::pybacked::PyBackedStr;

fn cid_hash_to_pydict<'py>(py: Python<'py>, cid: &Cid) -> Bound<'py, PyDict> {
    let hash = cid.hash();
    let dict_obj = PyDict::new(py);

    dict_obj.set_item("code", hash.code()).unwrap();
    dict_obj.set_item("size", hash.size()).unwrap();
    dict_obj
        .set_item("digest", PyBytes::new(py, &hash.digest()))
        .unwrap();

    dict_obj
}

fn cid_to_pydict<'py>(py: Python<'py>, cid: &Cid) -> Bound<'py, PyDict> {
    let dict_obj = PyDict::new(py);

    dict_obj.set_item("version", cid.version() as u64).unwrap();
    dict_obj.set_item("codec", cid.codec()).unwrap();
    dict_obj
        .set_item("hash", cid_hash_to_pydict(py, cid))
        .unwrap();

    dict_obj
}

fn decode_len(len: u64) -> Result<usize> {
    Ok(usize::try_from(len).map_err(|_| LengthOutOfRange::new::<usize>())?)
}

fn map_key_cmp(a: &Vec<u8>, b: &Vec<u8>) -> std::cmp::Ordering {
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

fn sort_map_keys(keys: &Bound<PyList>, len: usize) -> Result<Vec<(PyBackedStr, usize)>> {
    // Returns key and index.
    let mut keys_str = Vec::with_capacity(len);
    for i in 0..len {
        let item = keys.get_item(i)?;
        let key = match item.downcast::<PyString>() {
            Ok(k) => k.to_owned(),
            Err(_) => return Err(anyhow!("Map keys must be strings")),
        };
        let backed_str = match PyBackedStr::try_from(key) {
            Ok(bs) => bs,
            Err(_) => return Err(anyhow!("Failed to convert PyString to PyBackedStr")),
        };
        keys_str.push((backed_str, i));
    }

    if keys_str.len() < 2 {
        return Ok(keys_str);
    }

    keys_str.sort_by(|a, b| {
        // sort_unstable_by performs bad
        let (s1, _) = a;
        let (s2, _) = b;

        // sorted length-first by the byte representation of the string keys
        if s1.len() != s2.len() {
            s1.len().cmp(&s2.len())
        } else {
            s1.cmp(&s2)
        }
    });

    Ok(keys_str)
}

fn get_bytes_from_py_any<'py>(obj: &'py Bound<'py, PyAny>) -> PyResult<&'py [u8]> {
    if let Ok(b) = obj.downcast::<PyBytes>() {
        Ok(b.as_bytes())
    } else if let Ok(ba) = obj.downcast::<PyByteArray>() {
        Ok(unsafe { ba.as_bytes() })
    } else if let Ok(s) = obj.downcast::<PyString>() {
        Ok(s.to_str()?.as_bytes())
    } else {
        Err(get_err(
            "Failed to encode multibase",
            "Unsupported data type".to_string(),
        ))
    }
}

fn string_new_bound<'py>(py: Python<'py>, s: &[u8]) -> Bound<'py, PyString> {
    let ptr = s.as_ptr() as *const c_char;
    let len = s.len() as ffi::Py_ssize_t;
    unsafe {
        Bound::from_owned_ptr(py, ffi::PyUnicode_FromStringAndSize(ptr, len)).downcast_into_unchecked()
    }
}

fn decode_dag_cbor_to_pyobject<R: Read + Seek>(
    py: Python,
    r: &mut R,
    depth: usize,
) -> Result<PyObject> {
    unsafe {
        if depth > ffi::Py_GetRecursionLimit() as usize {
            PyErr::new::<pyo3::exceptions::PyRecursionError, _>(
                "RecursionError: maximum recursion depth exceeded in DAG-CBOR decoding",
            ).restore(py);

            return Err(anyhow!("Maximum recursion depth exceeded"));
        }
    }

    let major = decode::read_major(r)?;
    Ok(match major.kind() {
        MajorKind::UnsignedInt => decode::read_uint(r, major)?.into_pyobject(py)?.into(),
        MajorKind::NegativeInt => (-1 - decode::read_uint(r, major)? as i64).into_pyobject(py)?.into(),
        MajorKind::ByteString => {
            let len = decode::read_uint(r, major)?;
            PyBytes::new(py, &decode::read_bytes(r, len)?).into_pyobject(py)?.into()
        }
        MajorKind::TextString => {
            let len = decode::read_uint(r, major)?;
            string_new_bound(py, &decode::read_bytes(r, len)?).into_pyobject(py)?.into()
        }
        MajorKind::Array => {
            let len: ffi::Py_ssize_t = decode_len(decode::read_uint(r, major)?)?.try_into()?;

            unsafe {
                let ptr = ffi::PyList_New(len);

                for i in 0..len {
                    ffi::PyList_SET_ITEM(ptr, i, decode_dag_cbor_to_pyobject(py, r, depth + 1)?.into_ptr());
                }

                let list: Bound<'_, PyList> = Bound::from_owned_ptr(py, ptr).downcast_into_unchecked();
                list.into_pyobject(py)?.into()
            }
        }
        MajorKind::Map => {
            let len = decode_len(decode::read_uint(r, major)?)?;
            let dict = PyDict::new(py);

            let mut prev_key: Option<Vec<u8>> = None;
            for _ in 0..len {
                // DAG-CBOR keys are always strings
                let key_major = decode::read_major(r)?;
                if key_major.kind() != MajorKind::TextString {
                    return Err(anyhow!("Map keys must be strings"));
                }

                let key_len = decode::read_uint(r, key_major)?;
                let key = decode::read_bytes(r, key_len)?;

                if let Some(prev_key) = prev_key {
                    // it cares about duplicated keys too thanks to Ordering::Equal
                    if map_key_cmp(&prev_key, &key) != std::cmp::Ordering::Less {
                        return Err(anyhow!("Map keys must be sorted and unique"));
                    }
                }

                let key_py = string_new_bound(py, key.as_slice()).into_pyobject(py)?;
                prev_key = Some(key);

                let value_py = decode_dag_cbor_to_pyobject(py, r, depth + 1)?;
                dict.set_item(key_py, value_py)?;
            }

            dict.into_pyobject(py)?.into()
        }
        MajorKind::Tag => {
            let value = decode::read_uint(r, major)?;
            if value != 42 {
                return Err(anyhow!("Non-42 tags are not supported"));
            }

            // FIXME(MarshalX): to_bytes allocates
            let cid = decode::read_link(r)?.to_bytes();
            PyBytes::new(py, &cid).into_pyobject(py)?.into()
        }
        MajorKind::Other => match major {
            // FIXME(MarshalX): should be more clear for bool?
            cbor::FALSE => false.into_pyobject(py)?.into_any().unbind(),
            cbor::TRUE => true.into_pyobject(py)?.into_any().unbind(),
            cbor::NULL => py.None(),
            cbor::F32 => decode::read_f32(r)?.into_pyobject(py)?.into(),
            cbor::F64 => decode::read_f64(r)?.into_pyobject(py)?.into(),
            _ => return Err(anyhow!("Unsupported major type".to_string())),
        },
    })
}

fn encode_dag_cbor_from_pyobject<'py, W: Write>(
    py: Python<'py>,
    obj: &Bound<'py, PyAny>,
    w: &mut W,
) -> Result<()> {
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
        let buf = if obj.is_truthy()? {
            [cbor::TRUE.into()]
        } else {
            [cbor::FALSE.into()]
        };
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
    } else if let Ok(l) = obj.downcast::<PyList>() {
        let len = l.len();

        encode::write_u64(w, MajorKind::Array, len as u64)?;

        for i in 0..len {
            encode_dag_cbor_from_pyobject(py, &l.get_item(i)?, w)?;
        }

        Ok(())
    } else if let Ok(map) = obj.downcast::<PyDict>() {
        let len = map.len();
        let keys = sort_map_keys(&map.keys(), len)?;
        let values = map.values();

        encode::write_u64(w, MajorKind::Map, len as u64)?;

        for (key, i) in keys {
            let key_buf = key.as_bytes();
            encode::write_u64(w, MajorKind::TextString, key_buf.len() as u64)?;
            w.write_all(key_buf)?;

            encode_dag_cbor_from_pyobject(py, &values.get_item(i)?, w)?;
        }

        Ok(())
    } else if let Ok(f) = obj.downcast::<PyFloat>() {
        let v = f.value();
        if !v.is_finite() {
            return Err(NumberOutOfRange::new::<f64>().into());
        }

        let mut buf = [0xfb, 0, 0, 0, 0, 0, 0, 0, 0];
        BigEndian::write_f64(&mut buf[1..], v);
        w.write_all(&buf)?;

        Ok(())
    } else if let Ok(b) = obj.downcast::<PyBytes>() {
        // FIXME (MarshalX): it's not efficient to try to parse it as CID
        let cid = Cid::try_from(b.as_bytes());
        if let Ok(_) = cid {
            let buf = b.as_bytes();
            let len = buf.len();

            encode::write_tag(w, 42)?;
            encode::write_u64(w, MajorKind::ByteString, len as u64 + 1)?;
            w.write_all(&[0])?;
            w.write_all(&buf[..len])?;
        } else {
            let l: u64 = b.len()? as u64;

            encode::write_u64(w, MajorKind::ByteString, l)?;
            w.write_all(b.as_bytes())?;
        }

        Ok(())
    } else if let Ok(s) = obj.downcast::<PyString>() {
        let buf = s.to_str()?.as_bytes();

        encode::write_u64(w, MajorKind::TextString, buf.len() as u64)?;
        w.write_all(buf)?;

        Ok(())
    } else {
        Err(UnknownTag(0).into())
    }
}

#[pyfunction]
fn decode_dag_cbor_multi<'py>(py: Python<'py>, data: &[u8]) -> PyResult<Bound<'py, PyList>> {
    let mut reader = BufReader::new(Cursor::new(data));
    let decoded_parts = PyList::empty(py);

    loop {
        let py_object = decode_dag_cbor_to_pyobject(py, &mut reader, 0);
        if let Ok(py_object) = py_object {
            decoded_parts.append(py_object)?;
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
            return Err(anyhow!("Unexpected EOF while reading ULEB128 number."));
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
    let Ok(version) = read_u64_leb128(r) else {
        return Err(CidError::VarIntDecodeError);
    };
    let Ok(codec) = read_u64_leb128(r) else {
        return Err(CidError::VarIntDecodeError);
    };

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
pub fn decode_car<'py>(py: Python<'py>, data: &[u8]) -> PyResult<(PyObject, Bound<'py, PyDict>)> {
    let buf = &mut BufReader::new(Cursor::new(data));

    if let Err(_) = read_u64_leb128(buf) {
        return Err(get_err(
            "Failed to read CAR header",
            "Invalid uvarint".to_string(),
        ));
    }
    let Ok(header_obj) = decode_dag_cbor_to_pyobject(py, buf, 0) else {
        return Err(get_err(
            "Failed to read CAR header",
            "Invalid DAG-CBOR".to_string(),
        ));
    };

    let header = header_obj.downcast_bound::<PyDict>(py)?;

    let Some(version) = header.get_item("version")? else {
        return Err(get_err(
            "Failed to read CAR header",
            "Version is None".to_string(),
        ));
    };
    if version.downcast::<PyInt>()?.extract::<u64>()? != 1 {
        return Err(get_err(
            "Failed to read CAR header",
            "Unsupported version. Version must be 1".to_string(),
        ));
    }

    let Some(roots) = header.get_item("roots")? else {
        return Err(get_err(
            "Failed to read CAR header",
            "Roots is None".to_string(),
        ));
    };
    if roots.downcast::<PyList>()?.len() == 0 {
        return Err(get_err(
            "Failed to read CAR header",
            "Roots is empty. Must be at least one".to_string(),
        ));
    }

    // FIXME (MarshalX): we are not verifying if the roots are valid CIDs

    let parsed_blocks = PyDict::new(py);

    loop {
        if let Err(_) = read_u64_leb128(buf) {
            // FIXME (MarshalX): we are not raising an error here because of possible EOF
            break;
        }

        let cid_result = read_cid_from_bytes(buf);
        let Ok(cid) = cid_result else {
            return Err(get_err(
                "Failed to read CID of block",
                cid_result.unwrap_err().to_string(),
            ));
        };

        if cid.codec() != 0x71 {
            return Err(get_err(
                "Failed to read CAR block",
                "Unsupported codec. For now we support only DAG-CBOR (0x71)".to_string(),
            ));
        }

        let block_result = decode_dag_cbor_to_pyobject(py, buf, 0);
        let Ok(block) = block_result else {
            return Err(get_err(
                "Failed to read CAR block",
                block_result.unwrap_err().to_string(),
            ));
        };

        // FIXME(MarshalX): to_bytes allocates
        let key = PyBytes::new(py, &cid.to_bytes()).into_pyobject(py)?;
        parsed_blocks.set_item(key, block)?;
    }

    Ok((header_obj, parsed_blocks))
}

#[pyfunction]
pub fn decode_dag_cbor(py: Python, data: &[u8]) -> PyResult<PyObject> {
    let py_object = decode_dag_cbor_to_pyobject(py, &mut BufReader::new(Cursor::new(data)), 0);
    if let Ok(py_object) = py_object {
        Ok(py_object)
    } else {
        let err = get_err(
            "Failed to decode DAG-CBOR",
            py_object.unwrap_err().to_string(),
        );

        if let Some(py_err) = PyErr::take(py) {
            py_err.set_cause(py, Option::from(err));
            // in case something set global interpreter’s error,
            // for example C FFI function, we should return it
            // the real case: RecursionError (set by Py_EnterRecursiveCall)
            Err(py_err)
        } else {
            Err(err)
        }
    }
}

#[pyfunction]
pub fn encode_dag_cbor<'py>(
    py: Python<'py>,
    data: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyBytes>> {
    let mut buf = &mut BufWriter::new(Vec::new());
    if let Err(e) = encode_dag_cbor_from_pyobject(py, data, &mut buf) {
        return Err(get_err("Failed to encode DAG-CBOR", e.to_string()));
    }
    if let Err(e) = buf.flush() {
        return Err(get_err("Failed to flush buffer", e.to_string()));
    }
    Ok(PyBytes::new(py, &buf.get_ref()))
}

fn get_cid_from_py_any<'py>(data: &Bound<PyAny>) -> PyResult<Cid> {
    let cid: CidResult<Cid>;
    if let Ok(s) = data.downcast::<PyString>() {
        cid = Cid::try_from(s.to_str()?);
    } else {
        cid = Cid::try_from(get_bytes_from_py_any(data)?);
    }

    if let Ok(cid) = cid {
        Ok(cid)
    } else {
        Err(get_err(
            "Failed to decode CID",
            cid.unwrap_err().to_string(),
        ))
    }
}

#[pyfunction]
fn decode_cid<'py>(py: Python<'py>, data: &Bound<PyAny>) -> PyResult<Bound<'py, PyDict>> {
    Ok(cid_to_pydict(py, &get_cid_from_py_any(data)?))
}

#[pyfunction]
fn encode_cid<'py>(py: Python<'py>, data: &Bound<PyAny>) -> PyResult<Bound<'py, PyString>> {
    Ok(PyString::new(py, get_cid_from_py_any(data)?.to_string().as_str()))
}

#[pyfunction]
fn decode_multibase<'py>(py: Python<'py>, data: &str) -> PyResult<(char, Bound<'py, PyBytes>)> {
    let base = multibase::decode(data);
    if let Ok((base, data)) = base {
        Ok((base.code(), PyBytes::new(py, &data)))
    } else {
        Err(get_err(
            "Failed to decode multibase",
            base.unwrap_err().to_string(),
        ))
    }
}

#[pyfunction]
fn encode_multibase(code: char, data: &Bound<PyAny>) -> PyResult<String> {
    let data_bytes = get_bytes_from_py_any(data)?;
    let base = multibase::Base::from_code(code);
    if let Ok(base) = base {
        Ok(multibase::encode(base, data_bytes))
    } else {
        Err(get_err(
            "Failed to encode multibase",
            base.unwrap_err().to_string(),
        ))
    }
}

fn get_err(msg: &str, err: String) -> PyErr {
    PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}. {}", msg, err))
}

#[pymodule]
fn libipld(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(decode_cid, m)?)?;
    m.add_function(wrap_pyfunction!(encode_cid, m)?)?;

    m.add_function(wrap_pyfunction!(decode_car, m)?)?;

    m.add_function(wrap_pyfunction!(decode_dag_cbor, m)?)?;
    m.add_function(wrap_pyfunction!(decode_dag_cbor_multi, m)?)?;
    m.add_function(wrap_pyfunction!(encode_dag_cbor, m)?)?;

    m.add_function(wrap_pyfunction!(decode_multibase, m)?)?;
    m.add_function(wrap_pyfunction!(encode_multibase, m)?)?;

    Ok(())
}
