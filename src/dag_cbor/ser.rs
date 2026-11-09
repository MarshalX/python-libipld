use std::cell::Cell;

use anyhow::{anyhow, Result};
use pyo3::sync::critical_section::with_critical_section;
use pyo3::{ffi, prelude::*, types::*, Borrowed};

use crate::cid::{looks_like_cid, parse_cid_prefix};
use crate::error::{pending_or_value_error, recursion_error};
use crate::ffi::recursion::current_recursion_limit;
use crate::io::writer::{ARRAY, BYTES, MAP, NEGATIVE, STRING, TAG, UNSIGNED};
use crate::io::VecWriter;

#[derive(Clone, Copy)]
struct MapEntry {
    key: *const u8,
    key_len: usize,
    // First 8 key bytes as a big-endian integer (zero-padded), so sorting
    // rarely has to read the key itself.
    prefix: u64,
    value: *mut ffi::PyObject,
    #[cfg(Py_GIL_DISABLED)]
    key_obj: *mut ffi::PyObject,
}

impl MapEntry {
    #[inline]
    fn key(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.key, self.key_len) }
    }

    // DAG-CBOR map order: shorter keys first, then bytewise.
    #[inline]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.key_len, self.prefix)
            .cmp(&(other.key_len, other.prefix))
            .then_with(|| self.key().cmp(other.key()))
    }
}

#[inline]
fn key_prefix(key: &[u8]) -> u64 {
    match key.first_chunk::<8>() {
        Some(chunk) => u64::from_be_bytes(*chunk),
        None => key
            .iter()
            .enumerate()
            .fold(0, |prefix, (i, &b)| prefix | (b as u64) << (56 - 8 * i)),
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
    depth: usize,
    max_depth: usize,
}

impl Encoder {
    #[inline]
    fn enter(&mut self, py: Python<'_>) -> Result<()> {
        if self.depth > self.max_depth {
            return Err(recursion_error(
                py,
                "RecursionError: maximum recursion depth exceeded in DAG-CBOR encoding",
            ));
        }
        self.depth += 1;
        Ok(())
    }
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
fn list_item<'py>(l: &Bound<'py, PyList>, i: usize) -> Result<Bound<'py, PyAny>> {
    l.get_item(i)
        .map_err(|_| anyhow!("List changed size during encoding"))
}

#[inline]
fn encode_int(obj: &Bound<'_, PyAny>, w: &mut VecWriter) -> Result<()> {
    #[cfg(all(CPython, Py_3_12))]
    {
        if let Some((abs_val, neg)) = unsafe { crate::ffi::int::pylong_parts(obj.as_ptr()) } {
            if neg {
                w.head(NEGATIVE, abs_val - 1);
            } else {
                w.head(UNSIGNED, abs_val);
            }
            return Ok(());
        }
    }

    let i: i128 = obj.extract()?;
    if i.is_negative() {
        if -(i + 1) > u64::MAX as i128 {
            return Err(anyhow!("Number out of range"));
        }
        w.head(NEGATIVE, -(i + 1) as u64);
    } else {
        if i > u64::MAX as i128 {
            return Err(anyhow!("Number out of range"));
        }
        w.head(UNSIGNED, i as u64);
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
                let key_bytes = std::slice::from_raw_parts(utf8.cast::<u8>(), len as usize);
                enc.entries.0.push(MapEntry {
                    key: key_bytes.as_ptr(),
                    key_len: key_bytes.len(),
                    prefix: key_prefix(key_bytes),
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
        entries.sort_unstable_by(MapEntry::cmp);
    }

    let len = entries.len();
    enc.w.head(MAP, len as u64);
    for i in base..base + len {
        let entry = enc.entries.0[i];
        enc.w.segment(STRING, entry.key());
        let value = unsafe { Borrowed::from_ptr(py, entry.value) };
        encode_value(py, &value, enc)?;
    }
    Ok(())
}

fn encode_map<'py>(py: Python<'py>, map: &Bound<'py, PyDict>, enc: &mut Encoder) -> Result<()> {
    enc.enter(py)?;
    let base = enc.entries.0.len();
    let result = encode_map_entries(py, map, enc, base);
    enc.depth -= 1;
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

#[inline(always)]
fn encode_value<'py>(py: Python<'py>, obj: &Bound<'py, PyAny>, enc: &mut Encoder) -> Result<()> {
    // Exact-type pointer compare per branch avoids the MRO walk that
    // `is_instance_of` / `cast` perform. Order tuned for typical ATProto
    // record shapes; subclasses fall through to the slow path.
    let tp = unsafe { ffi::Py_TYPE(obj.as_ptr()) };
    unsafe {
        if tp == &raw mut ffi::PyUnicode_Type {
            let s = obj.cast_unchecked::<PyString>();
            enc.w.segment(STRING, s.to_str()?.as_bytes());
            return Ok(());
        }
        if tp == &raw mut ffi::PyDict_Type {
            return encode_map(py, obj.cast_unchecked::<PyDict>(), enc);
        }
        if tp == &raw mut ffi::PyList_Type {
            return encode_list(py, obj.cast_unchecked::<PyList>(), enc);
        }
        if tp == &raw mut ffi::PyLong_Type {
            return encode_int(obj, &mut enc.w);
        }
        if tp == &raw mut ffi::PyBytes_Type {
            return encode_bytes(obj.cast_unchecked::<PyBytes>(), &mut enc.w);
        }
        if tp == &raw mut ffi::PyBool_Type {
            enc.w.bool(obj.as_ptr() == ffi::Py_True());
            return Ok(());
        }
        if obj.as_ptr() == ffi::Py_None() {
            enc.w.null();
            return Ok(());
        }
        if tp == &raw mut ffi::PyFloat_Type {
            return encode_float(obj.cast_unchecked::<PyFloat>(), &mut enc.w);
        }
    }

    encode_subclass(py, obj, enc)
}

#[inline(never)]
fn encode_list<'py>(py: Python<'py>, l: &Bound<'py, PyList>, enc: &mut Encoder) -> Result<()> {
    enc.enter(py)?;
    let len = l.len();
    enc.w.head(ARRAY, len as u64);
    for i in 0..len {
        #[cfg(not(Py_GIL_DISABLED))]
        let item = unsafe { list_item(l, i) };
        #[cfg(Py_GIL_DISABLED)]
        let item = list_item(l, i)?;
        encode_value(py, &item, enc)?;
    }
    enc.depth -= 1;
    Ok(())
}

#[inline]
fn encode_bytes(b: &Bound<'_, PyBytes>, w: &mut VecWriter) -> Result<()> {
    let bytes = b.as_bytes();
    if looks_like_cid(bytes) && parse_cid_prefix(bytes).is_some() {
        // Tag 42 around a byte string holding 0x00 followed by the CID.
        w.head(TAG, 42);
        w.head(BYTES, 1 + bytes.len() as u64);
        w.raw(&[0x00]);
        w.raw(bytes);
    } else {
        w.segment(BYTES, bytes);
    }
    Ok(())
}

#[inline]
fn encode_float(f: &Bound<'_, PyFloat>, w: &mut VecWriter) -> Result<()> {
    let v = f.value();
    if !v.is_finite() {
        return Err(anyhow!("Number out of range"));
    }
    // DAG-CBOR forbids -0.0; it equals 0.0, so encode it as 0.0.
    let v = if v == 0.0 { 0.0 } else { v };
    w.f64(v);
    Ok(())
}

// Subclasses of supported types (rare in DAG-CBOR usage).
#[cold]
#[inline(never)]
fn encode_subclass<'py>(py: Python<'py>, obj: &Bound<'py, PyAny>, enc: &mut Encoder) -> Result<()> {
    if obj.is_instance_of::<PyBool>() {
        enc.w.bool(obj.as_ptr() == unsafe { ffi::Py_True() });
        Ok(())
    } else if obj.is_instance_of::<PyInt>() {
        encode_int(obj, &mut enc.w)
    } else if let Ok(l) = obj.cast::<PyList>() {
        encode_list(py, l, enc)
    } else if let Ok(map) = obj.cast::<PyDict>() {
        encode_map(py, map, enc)
    } else if let Ok(s) = obj.cast::<PyString>() {
        enc.w.segment(STRING, s.to_str()?.as_bytes());
        Ok(())
    } else if let Ok(b) = obj.cast::<PyBytes>() {
        encode_bytes(b, &mut enc.w)
    } else if let Ok(f) = obj.cast::<PyFloat>() {
        encode_float(f, &mut enc.w)
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
        depth: 0,
        max_depth: current_recursion_limit(),
    };
    if let Err(e) = encode_value(py, data, &mut enc) {
        return Err(pending_or_value_error(py, "Failed to encode DAG-CBOR", e));
    }
    Ok(PyBytes::new(py, enc.w.as_slice()))
}
