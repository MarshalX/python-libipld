use std::cell::Cell;

use anyhow::{anyhow, Result};
use cbor4ii::core::{
    enc::{self, Encode},
    types,
};
use pyo3::sync::critical_section::with_critical_section;
use pyo3::{ffi, prelude::*, types::*, Borrowed};

use crate::cid::{looks_like_cid, parse_cid_prefix};
use crate::error::value_error;
use crate::io::VecWriter;

struct PrefixedCidBytes<'a>(&'a [u8]);

impl<'a> Encode for PrefixedCidBytes<'a> {
    fn encode<W: enc::Write>(&self, w: &mut W) -> Result<(), enc::Error<W::Error>> {
        // length prefix for bytes: 1 (leading 0) + payload
        types::Bytes::bounded(1 + self.0.len(), w)?;
        w.push(&[0x00])?;
        w.push(self.0)?;
        Ok(())
    }
}

#[derive(Clone, Copy)]
struct MapEntry {
    key: *const u8,
    key_len: usize,
    value: *mut ffi::PyObject,
    #[cfg(Py_GIL_DISABLED)]
    key_obj: *mut ffi::PyObject,
}

impl MapEntry {
    #[inline]
    fn key(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.key, self.key_len) }
    }
}

const MAX_POOLED_ENTRIES: usize = 1 << 14;

thread_local! {
    static ENTRY_POOL: Cell<Vec<MapEntry>> = const { Cell::new(Vec::new()) };
}

struct MapEntries(Vec<MapEntry>);

impl MapEntries {
    #[inline]
    fn new() -> Self {
        let mut entries = ENTRY_POOL.take();
        entries.clear();
        MapEntries(entries)
    }
}

impl Drop for MapEntries {
    fn drop(&mut self) {
        let entries = std::mem::take(&mut self.0);
        if entries.capacity() <= MAX_POOLED_ENTRIES {
            ENTRY_POOL.set(entries);
        }
    }
}

struct Encoder {
    w: VecWriter,
    entries: MapEntries,
}

#[cfg(not(Py_GIL_DISABLED))]
#[inline]
unsafe fn list_item<'a, 'py>(l: &'a Bound<'py, PyList>, i: usize) -> Borrowed<'a, 'py, PyAny> {
    Borrowed::from_ptr(
        l.py(),
        ffi::PyList_GET_ITEM(l.as_ptr(), i as ffi::Py_ssize_t),
    )
}

#[cfg(Py_GIL_DISABLED)]
#[inline]
unsafe fn list_item<'py>(l: &Bound<'py, PyList>, i: usize) -> Bound<'py, PyAny> {
    l.get_item_unchecked(i)
}

#[inline]
fn encode_int<W: enc::Write>(obj: &Bound<'_, PyAny>, w: &mut W) -> Result<()>
where
    W::Error: Send + Sync,
{
    #[cfg(all(CPython, Py_3_12))]
    {
        if let Some((abs_val, neg)) = unsafe { crate::ffi::int::pylong_parts(obj.as_ptr()) } {
            if neg {
                types::Negative(abs_val - 1).encode(w)?;
            } else {
                abs_val.encode(w)?;
            }
            return Ok(());
        }
    }

    let i: i128 = obj.extract()?;
    if i.is_negative() {
        if -(i + 1) > u64::MAX as i128 {
            return Err(anyhow!("Number out of range"));
        }
        types::Negative(-(i + 1) as u64).encode(w)?;
    } else {
        if i > u64::MAX as i128 {
            return Err(anyhow!("Number out of range"));
        }
        (i as u64).encode(w)?;
    }
    Ok(())
}

fn collect_map_entries(py: Python<'_>, map: &Bound<'_, PyDict>, enc: &mut Encoder) -> Result<()> {
    with_critical_section(map, || {
        let mut pos: ffi::Py_ssize_t = 0;
        let mut key: *mut ffi::PyObject = std::ptr::null_mut();
        let mut value: *mut ffi::PyObject = std::ptr::null_mut();
        while unsafe { ffi::PyDict_Next(map.as_ptr(), &mut pos, &mut key, &mut value) } != 0 {
            unsafe {
                if ffi::PyUnicode_Check(key) == 0 {
                    return Err(anyhow!("Map keys must be strings"));
                }
                let mut len: ffi::Py_ssize_t = 0;
                let utf8 = ffi::PyUnicode_AsUTF8AndSize(key, &mut len);
                if utf8.is_null() {
                    return Err(anyhow!(PyErr::fetch(py)));
                }
                #[cfg(Py_GIL_DISABLED)]
                {
                    ffi::Py_INCREF(key);
                    ffi::Py_INCREF(value);
                }
                enc.entries.0.push(MapEntry {
                    key: utf8.cast(),
                    key_len: len as usize,
                    value,
                    #[cfg(Py_GIL_DISABLED)]
                    key_obj: key,
                });
            }
        }
        Ok(())
    })
}

fn encode_map_entries<'py>(
    py: Python<'py>,
    map: &Bound<'py, PyDict>,
    enc: &mut Encoder,
    base: usize,
) -> Result<()> {
    collect_map_entries(py, map, enc)?;

    let entries = &mut enc.entries.0[base..];
    if entries.len() >= 2 {
        // Keys are unique, so stability buys nothing.
        entries.sort_unstable_by(|a, b| {
            if a.key_len != b.key_len {
                a.key_len.cmp(&b.key_len)
            } else {
                a.key().cmp(b.key())
            }
        });
    }

    let len = entries.len();
    types::Map::bounded(len, &mut enc.w)?;
    for i in base..base + len {
        let entry = enc.entries.0[i];
        // CPython hands out the UTF-8 buffer of a valid `str`.
        unsafe { std::str::from_utf8_unchecked(entry.key()) }.encode(&mut enc.w)?;
        let value = unsafe { Borrowed::from_ptr(py, entry.value) };
        from_pyobject(py, &value, enc)?;
    }
    Ok(())
}

fn encode_map<'py>(py: Python<'py>, map: &Bound<'py, PyDict>, enc: &mut Encoder) -> Result<()> {
    let base = enc.entries.0.len();
    let result = encode_map_entries(py, map, enc, base);
    // Always unwind this level's slice of the shared stack, also on error.
    #[cfg(Py_GIL_DISABLED)]
    for entry in &enc.entries.0[base..] {
        unsafe {
            ffi::Py_DECREF(entry.key_obj);
            ffi::Py_DECREF(entry.value);
        }
    }
    enc.entries.0.truncate(base);
    result
}

fn from_pyobject<'py>(py: Python<'py>, obj: &Bound<'py, PyAny>, enc: &mut Encoder) -> Result<()> {
    // Exact-type pointer compare per branch avoids the MRO walk that
    // `is_instance_of` / `cast` perform. Order tuned for typical ATProto
    // record shapes; subclasses fall through to the slow path below.
    let tp = unsafe { ffi::Py_TYPE(obj.as_ptr()) };
    unsafe {
        if tp == &raw mut ffi::PyUnicode_Type {
            let s = obj.cast_unchecked::<PyString>();
            s.to_str()?.encode(&mut enc.w)?;
            return Ok(());
        }
        if tp == &raw mut ffi::PyDict_Type {
            return encode_map(py, obj.cast_unchecked::<PyDict>(), enc);
        }
        if tp == &raw mut ffi::PyList_Type {
            let l = obj.cast_unchecked::<PyList>();
            let len = l.len();
            types::Array::bounded(len, &mut enc.w)?;
            for i in 0..len {
                let item = list_item(l, i);
                from_pyobject(py, &item, enc)?;
            }
            return Ok(());
        }
        if tp == &raw mut ffi::PyLong_Type {
            return encode_int(obj, &mut enc.w);
        }
        if tp == &raw mut ffi::PyBytes_Type {
            let b = obj.cast_unchecked::<PyBytes>();
            let bytes = b.as_bytes();
            if looks_like_cid(bytes) && parse_cid_prefix(bytes).is_some() {
                // by providing custom encoding we avoid extra allocation
                types::Tag(42, PrefixedCidBytes(bytes)).encode(&mut enc.w)?;
            } else {
                types::Bytes(bytes).encode(&mut enc.w)?;
            }
            return Ok(());
        }
        if tp == &raw mut ffi::PyBool_Type {
            (obj.as_ptr() == ffi::Py_True()).encode(&mut enc.w)?;
            return Ok(());
        }
        if obj.as_ptr() == ffi::Py_None() {
            types::Null.encode(&mut enc.w)?;
            return Ok(());
        }
        if tp == &raw mut ffi::PyFloat_Type {
            let f = obj.cast_unchecked::<PyFloat>();
            let v = f.value();
            if !v.is_finite() {
                return Err(anyhow!("Number out of range"));
            }
            v.encode(&mut enc.w)?;
            return Ok(());
        }
    }

    // Slow path: subclasses of supported types (rare in DAG-CBOR usage).
    if obj.is_instance_of::<PyBool>() {
        (obj.as_ptr() == unsafe { ffi::Py_True() }).encode(&mut enc.w)?;
        Ok(())
    } else if obj.is_instance_of::<PyInt>() {
        encode_int(obj, &mut enc.w)
    } else if let Ok(l) = obj.cast::<PyList>() {
        let len = l.len();
        types::Array::bounded(len, &mut enc.w)?;
        for i in 0..len {
            let item = unsafe { list_item(l, i) };
            from_pyobject(py, &item, enc)?;
        }
        Ok(())
    } else if let Ok(map) = obj.cast::<PyDict>() {
        encode_map(py, map, enc)
    } else if let Ok(s) = obj.cast::<PyString>() {
        s.to_str()?.encode(&mut enc.w)?;
        Ok(())
    } else if let Ok(b) = obj.cast::<PyBytes>() {
        let bytes = b.as_bytes();
        if looks_like_cid(bytes) && parse_cid_prefix(bytes).is_some() {
            types::Tag(42, PrefixedCidBytes(bytes)).encode(&mut enc.w)?;
        } else {
            types::Bytes(bytes).encode(&mut enc.w)?;
        }
        Ok(())
    } else if let Ok(f) = obj.cast::<PyFloat>() {
        let v = f.value();
        if !v.is_finite() {
            return Err(anyhow!("Number out of range"));
        }
        v.encode(&mut enc.w)?;
        Ok(())
    } else {
        Err(anyhow!("Unknown tag"))
    }
}

#[pyfunction]
pub fn encode_dag_cbor<'py>(
    py: Python<'py>,
    data: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyBytes>> {
    let mut enc = Encoder {
        w: VecWriter::new(),
        entries: MapEntries::new(),
    };
    if let Err(e) = from_pyobject(py, data, &mut enc) {
        return Err(value_error("Failed to encode DAG-CBOR", e.to_string()));
    }
    Ok(PyBytes::new(py, enc.w.as_slice()))
}
