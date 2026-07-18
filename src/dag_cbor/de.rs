use anyhow::{anyhow, Result};
use cbor4ii::core::{
    dec::{self, Read},
    major, marker,
};
use pyo3::{ffi, prelude::*, types::*, BoundObject};

use crate::cid::parse_cid_prefix;
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

// Argument of the already-peeked header byte: low bits 0..=0x17 are the value
// itself; 0x18..=0x1b mean 1/2/4/8 following big-endian bytes. Consumes the
// header. Rejects indefinite-length and reserved arguments (0x1c..=0x1f),
// which DAG-CBOR forbids.
#[inline]
fn decode_arg<'de, R: dec::Read<'de>>(r: &mut R, byte: u8) -> Result<u64>
where
    R::Error: Send + Sync,
{
    r.advance(1);
    let low = byte & 0x1f;
    if low <= 0x17 {
        return Ok(low as u64);
    }
    let n = match low {
        0x18 => 1,
        0x19 => 2,
        0x1a => 4,
        0x1b => 8,
        _ => return Err(anyhow!("Indefinite or reserved header argument")),
    };
    let buf = r.fill(n)?;
    let s = buf.as_ref();
    if s.len() < n {
        return Err(anyhow!("end of data"));
    }
    let mut be = [0u8; 8];
    be[8 - n..].copy_from_slice(&s[..n]);
    r.advance(n);
    Ok(u64::from_be_bytes(be))
}

// Definite-length bytes/string payload, zero-copy from the input.
#[inline]
fn decode_seg<'de, R: dec::Read<'de>>(r: &mut R, byte: u8) -> Result<&'de [u8]>
where
    R::Error: Send + Sync,
{
    let len = usize::try_from(decode_arg(r, byte)?)?;
    match r.fill(len)? {
        dec::Reference::Long(s) if s.len() >= len => {
            let s = &s[..len];
            r.advance(len);
            Ok(s)
        }
        _ => Err(anyhow!("end of data")),
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
        major::UNSIGNED => decode_arg(r, byte)?.into_pyobject(py)?.into(),
        major::NEGATIVE => {
            let v = decode_arg(r, byte)?;
            // `-1 - v` fits an i64 for v < 2^63; the i128 fallback covers the
            // rest and costs a `_PyLong_FromByteArray`-style conversion.
            if v <= i64::MAX as u64 {
                (-1i64 - v as i64).into_pyobject(py)?.into()
            } else {
                (-1i128 - v as i128).into_pyobject(py)?.into()
            }
        }
        major::BYTES => PyBytes::new(py, decode_seg(r, byte)?)
            .into_pyobject(py)?
            .into(),
        major::STRING => {
            // ASCII fast path inside the helper; non-ASCII falls through to
            // `PyUnicode_DecodeUTF8`, which is where the spec validation lives.
            from_bytes(py, decode_seg(r, byte)?)?.into()
        }
        major::ARRAY => {
            let len = usize::try_from(decode_arg(r, byte)?)?;
            // Every element costs at least one byte; reject a claimed length
            // beyond the remaining input before allocating for it.
            if r.fill(len)?.as_ref().len() < len {
                return Err(anyhow!("Array length exceeds remaining data"));
            }
            let len: ffi::Py_ssize_t = len.try_into()?;

            unsafe {
                let ptr = ffi::PyList_New(len);
                if ptr.is_null() {
                    return Err(anyhow!(PyErr::fetch(py)));
                }
                // Owned before filling so an error mid-fill releases the list;
                // list dealloc tolerates the remaining NULL slots.
                let list: Bound<'_, PyList> = Bound::from_owned_ptr(py, ptr).cast_into_unchecked();

                for i in 0..len {
                    ffi::PyList_SET_ITEM(
                        ptr,
                        i,
                        to_pyobject(py, r, depth + 1, max_depth)?.into_ptr(),
                    );
                }

                list.into_pyobject(py)?.into()
            }
        }
        major::MAP => {
            let len = usize::try_from(decode_arg(r, byte)?)?;
            // Every entry costs at least two bytes (key + value); reject a
            // claimed length beyond the remaining input before presizing.
            let need = len.saturating_mul(2);
            if r.fill(need)?.as_ref().len() < need {
                return Err(anyhow!("Map length exceeds remaining data"));
            }
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
                let key_byte = peek_one(r)?;
                if dec::if_major(key_byte) != major::STRING {
                    return Err(anyhow!("Map keys must be strings"));
                }
                let key = decode_seg(r, key_byte)?;

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
            let value = decode_arg(r, byte)?;
            if value != 42 {
                return Err(anyhow!("Non-42 tags are not supported"));
            }

            let content_byte = peek_one(r)?;
            if dec::if_major(content_byte) != major::BYTES {
                return Err(anyhow!("Invalid CID"));
            }
            let cid = decode_seg(r, content_byte)?;

            // we expect CIDs to have a leading zero byte
            if cid.len() <= 1 || cid[0] != 0 {
                return Err(anyhow!("Invalid CID"));
            }

            let cid_without_prefix = &cid[1..];
            if parse_cid_prefix(cid_without_prefix).is_none() {
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
                let value = f32::from_bits(decode_arg(r, byte)? as u32);
                if !value.is_finite() {
                    return Err(anyhow!(
                        "Number out of range for f32 (NaNs are forbidden)".to_string()
                    ));
                }
                value.into_pyobject(py)?.into()
            }
            marker::F64 => {
                let value = f64::from_bits(decode_arg(r, byte)?);
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

// Wrap a decode failure; an error already set on the interpreter (e.g. the
// RecursionError `restore`d above) wins, with the decode error as its cause.
fn decode_error(py: Python, e: anyhow::Error) -> PyErr {
    let err = value_error("Failed to decode DAG-CBOR", e.to_string());
    if let Some(py_err) = PyErr::take(py) {
        py_err.set_cause(py, Option::from(err));
        py_err
    } else {
        err
    }
}

#[pyfunction]
pub fn decode_dag_cbor_multi<'py>(py: Python<'py>, data: &[u8]) -> PyResult<Bound<'py, PyList>> {
    let mut reader = SliceReader::new(data);
    let decoded_parts = PyList::empty(py);
    let max_depth = current_recursion_limit();

    while !reader.fill(1)?.as_ref().is_empty() {
        match to_pyobject(py, &mut reader, 0, max_depth) {
            Ok(py_object) => decoded_parts.append(py_object)?,
            Err(e) => return Err(decode_error(py, e)),
        }
    }

    Ok(decoded_parts)
}

#[pyfunction]
pub fn decode_dag_cbor(py: Python, data: &[u8]) -> PyResult<Py<PyAny>> {
    let mut reader = SliceReader::new(data);
    let max_depth = current_recursion_limit();
    match to_pyobject(py, &mut reader, 0, max_depth) {
        Ok(py_object) => {
            // check for any remaining data in the reader
            if reader.fill(1)?.as_ref().is_empty() {
                Ok(py_object)
            } else {
                Err(value_error(
                    "Failed to decode DAG-CBOR",
                    "Invalid DAG-CBOR: contains multiple objects (CBOR sequence)".to_string(),
                ))
            }
        }
        Err(e) => Err(decode_error(py, e)),
    }
}
