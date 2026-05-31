use anyhow::{anyhow, Result};
use cbor4ii::core::{
    dec::{self, Decode, Read},
    major, marker, types,
};
use pyo3::{ffi, prelude::*, types::*, BoundObject};

use crate::error::value_error;
use crate::ffi::dict::new_presized;
use crate::ffi::key_cache::intern;
use crate::ffi::recursion::current_recursion_limit;
use crate::ffi::string::from_bytes;
use crate::io::{peek_one, SliceReader};

#[cfg(CPython)]
use crate::ffi::dict::set_item_known_hash;

fn map_key_cmp(a: &[u8], b: &[u8]) -> std::cmp::Ordering {
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

pub(crate) fn to_pyobject<'de, R: dec::Read<'de>>(
    py: Python,
    r: &mut R,
    depth: usize,
    max_depth: usize,
) -> Result<Py<PyAny>>
where
    R::Error: Send + Sync,
{
    if depth > max_depth {
        PyErr::new::<pyo3::exceptions::PyRecursionError, _>(
            "RecursionError: maximum recursion depth exceeded in DAG-CBOR decoding",
        )
        .restore(py);

        return Err(anyhow!("Maximum recursion depth exceeded"));
    }

    let byte = peek_one(r)?;
    Ok(match dec::if_major(byte) {
        major::UNSIGNED => u64::decode(r)?.into_pyobject(py)?.into(),
        major::NEGATIVE => i128::decode(r)?.into_pyobject(py)?.into(),
        major::BYTES => PyBytes::new(py, <types::Bytes<&[u8]>>::decode(r)?.0)
            .into_pyobject(py)?
            .into(),
        major::STRING => {
            // ASCII fast path inside the helper; non-ASCII falls through to
            // `PyUnicode_DecodeUTF8`, which is where the spec validation lives.
            from_bytes(
                py,
                <types::UncheckedStr<&[u8]>>::decode(r)
                    .map_err(|_| anyhow!("Cannot decode as bytes"))?
                    .0,
            )?
            .into()
        }
        major::ARRAY => {
            let len: ffi::Py_ssize_t = types::Array::len(r)?
                .ok_or_else(|| anyhow!("Array must contain length"))?
                .try_into()?;

            unsafe {
                let ptr = ffi::PyList_New(len);

                for i in 0..len {
                    ffi::PyList_SET_ITEM(
                        ptr,
                        i,
                        to_pyobject(py, r, depth + 1, max_depth)?.into_ptr(),
                    );
                }

                let list: Bound<'_, PyList> = Bound::from_owned_ptr(py, ptr).cast_into_unchecked();
                list.into_pyobject(py)?.into()
            }
        }
        major::MAP => {
            let len = types::Map::len(r)?.ok_or_else(|| anyhow!("Map must contain length"))?;
            // Length is known up front; presize to avoid rehashes as we fill.
            let dict = unsafe {
                let ptr = new_presized(len);
                if ptr.is_null() {
                    return Err(anyhow!(PyErr::fetch(py)));
                }
                Bound::from_owned_ptr(py, ptr).cast_into_unchecked::<PyDict>()
            };

            let mut prev_key: Option<&[u8]> = None;
            for _ in 0..len {
                // DAG-CBOR keys are always strings. Python does the UTF-8 validation when creating
                // the string.
                let key = <types::UncheckedStr<&[u8]>>::decode(r)
                    .map_err(|_| anyhow!("Map keys must be strings"))?
                    .0;

                if let Some(prev_key) = prev_key {
                    // it cares about duplicated keys too thanks to Ordering::Equal
                    if map_key_cmp(prev_key, key) != std::cmp::Ordering::Less {
                        return Err(anyhow!("Map keys must be sorted and unique"));
                    }
                }

                prev_key = Some(key);

                let (key_ptr, key_hash) = unsafe { intern(py, key)? };
                let key_bound: Bound<'_, PyAny> = unsafe { Bound::from_owned_ptr(py, key_ptr) };

                let value_py = to_pyobject(py, r, depth + 1, max_depth)?;

                #[cfg(CPython)]
                unsafe {
                    set_item_known_hash(py, &dict, &key_bound, value_py, key_hash)?;
                }
                #[cfg(not(CPython))]
                {
                    let _ = key_hash;
                    dict.set_item(&key_bound, value_py)?;
                }
            }

            dict.into_pyobject(py)?.into()
        }
        major::TAG => {
            let value = types::Tag::tag(r)?;
            if value != 42 {
                return Err(anyhow!("Non-42 tags are not supported"));
            }

            let cid = <types::Bytes<&[u8]>>::decode(r)?.0;

            // we expect CIDs to have a leading zero byte
            if cid.len() <= 1 || cid[0] != 0 {
                return Err(anyhow!("Invalid CID"));
            }

            let cid_without_prefix = &cid[1..];
            if ::cid::Cid::try_from(cid_without_prefix).is_err() {
                return Err(anyhow!("Invalid CID"));
            }

            PyBytes::new(py, cid_without_prefix)
                .into_pyobject(py)?
                .into()
        }
        major::SIMPLE => match byte {
            // FIXME(MarshalX): should be more clear for bool?
            marker::FALSE => {
                r.advance(1);
                false.into_pyobject(py)?.into_any().unbind()
            }
            marker::TRUE => {
                r.advance(1);
                true.into_pyobject(py)?.into_any().unbind()
            }
            marker::NULL => {
                r.advance(1);
                py.None()
            }
            marker::F32 => {
                let value = f32::decode(r)?;
                if !value.is_finite() {
                    return Err(anyhow!(
                        "Number out of range for f32 (NaNs are forbidden)".to_string()
                    ));
                }
                value.into_pyobject(py)?.into()
            }
            marker::F64 => {
                let value = f64::decode(r)?;
                if !value.is_finite() {
                    return Err(anyhow!(
                        "Number out of range for f64 (NaNs are forbidden)".to_string()
                    ));
                }
                value.into_pyobject(py)?.into()
            }
            _ => return Err(anyhow!("Unsupported major type".to_string())),
        },
        _ => return Err(anyhow!("Invalid major type".to_string())),
    })
}

#[pyfunction]
pub fn decode_dag_cbor_multi<'py>(py: Python<'py>, data: &[u8]) -> PyResult<Bound<'py, PyList>> {
    let mut reader = SliceReader::new(data);
    let decoded_parts = PyList::empty(py);
    let max_depth = current_recursion_limit();

    loop {
        let py_object = to_pyobject(py, &mut reader, 0, max_depth);
        if let Ok(py_object) = py_object {
            decoded_parts.append(py_object)?;
        } else {
            break;
        }
    }

    Ok(decoded_parts)
}

#[pyfunction]
pub fn decode_dag_cbor(py: Python, data: &[u8]) -> PyResult<Py<PyAny>> {
    let mut reader = SliceReader::new(data);
    let max_depth = current_recursion_limit();
    let py_object = to_pyobject(py, &mut reader, 0, max_depth);
    if let Ok(py_object) = py_object {
        // check for any remaining data in the reader
        if reader.fill(1)?.as_ref().is_empty() {
            Ok(py_object)
        } else {
            Err(value_error(
                "Failed to decode DAG-CBOR",
                "Invalid DAG-CBOR: contains multiple objects (CBOR sequence)".to_string(),
            ))
        }
    } else {
        let err = value_error(
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
