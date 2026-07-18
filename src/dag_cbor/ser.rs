use anyhow::{anyhow, Result};
use cbor4ii::core::{
    enc::{self, Encode},
    types,
};
use pyo3::pybacked::PyBackedStr;
use pyo3::{ffi, prelude::*, types::*};

use crate::cid::looks_like_cid;
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

// One dict walk collects (key, value) pairs together; sorting by-index and
// re-fetching values through `map.values()` would materialize two extra
// PyLists and walk the dict three times.
fn sorted_map_entries<'py>(
    map: &Bound<'py, PyDict>,
) -> Result<Vec<(PyBackedStr, Bound<'py, PyAny>)>> {
    let len = map.len();
    let mut entries: Vec<(PyBackedStr, Bound<'py, PyAny>)> = Vec::with_capacity(len);

    for (key, value) in map.iter() {
        let key_str = match key.cast_into::<PyString>() {
            Ok(k) => k,
            Err(_) => return Err(anyhow!("Map keys must be strings")),
        };
        let backed = PyBackedStr::try_from(key_str)
            .map_err(|_| anyhow!("Failed to convert PyString to PyBackedStr"))?;
        entries.push((backed, value));
    }

    if entries.len() >= 2 {
        entries.sort_by(|a, b| {
            // sort_unstable_by performs bad in past benchmarks; revisit if data shape changes.
            let (s1, _) = a;
            let (s2, _) = b;
            if s1.len() != s2.len() {
                s1.len().cmp(&s2.len())
            } else {
                s1.as_bytes().cmp(s2.as_bytes())
            }
        });
    }

    Ok(entries)
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

fn from_pyobject<'py, W: enc::Write>(
    _py: Python<'py>,
    obj: &Bound<'py, PyAny>,
    w: &mut W,
) -> Result<()>
where
    W::Error: Send + Sync,
{
    // Exact-type pointer compare per branch avoids the MRO walk that
    // `is_instance_of` / `cast` perform. Order tuned for typical ATProto
    // record shapes; subclasses fall through to the slow path below.
    let tp = unsafe { ffi::Py_TYPE(obj.as_ptr()) };
    unsafe {
        if tp == &raw mut ffi::PyUnicode_Type {
            let s = obj.cast_unchecked::<PyString>();
            s.to_str()?.encode(w)?;
            return Ok(());
        }
        if tp == &raw mut ffi::PyDict_Type {
            let map = obj.cast_unchecked::<PyDict>();
            let entries = sorted_map_entries(map)?;
            types::Map::bounded(entries.len(), w)?;
            for (key, value) in &entries {
                (&**key).encode(w)?;
                from_pyobject(_py, value, w)?;
            }
            return Ok(());
        }
        if tp == &raw mut ffi::PyList_Type {
            let l = obj.cast_unchecked::<PyList>();
            let len = l.len();
            types::Array::bounded(len, w)?;
            for i in 0..len {
                let item = l.get_item_unchecked(i);
                from_pyobject(_py, &item, w)?;
            }
            return Ok(());
        }
        if tp == &raw mut ffi::PyLong_Type {
            return encode_int(obj, w);
        }
        if tp == &raw mut ffi::PyBytes_Type {
            let b = obj.cast_unchecked::<PyBytes>();
            let bytes = b.as_bytes();
            if looks_like_cid(bytes) && ::ipld_core::cid::Cid::try_from(bytes).is_ok() {
                // by providing custom encoding we avoid extra allocation
                types::Tag(42, PrefixedCidBytes(bytes)).encode(w)?;
            } else {
                types::Bytes(bytes).encode(w)?;
            }
            return Ok(());
        }
        if tp == &raw mut ffi::PyBool_Type {
            (obj.as_ptr() == ffi::Py_True()).encode(w)?;
            return Ok(());
        }
        if obj.as_ptr() == ffi::Py_None() {
            types::Null.encode(w)?;
            return Ok(());
        }
        if tp == &raw mut ffi::PyFloat_Type {
            let f = obj.cast_unchecked::<PyFloat>();
            let v = f.value();
            if !v.is_finite() {
                return Err(anyhow!("Number out of range"));
            }
            v.encode(w)?;
            return Ok(());
        }
    }

    // Slow path: subclasses of supported types (rare in DAG-CBOR usage).
    if obj.is_instance_of::<PyBool>() {
        (obj.as_ptr() == unsafe { ffi::Py_True() }).encode(w)?;
        Ok(())
    } else if obj.is_instance_of::<PyInt>() {
        encode_int(obj, w)
    } else if let Ok(l) = obj.cast::<PyList>() {
        let len = l.len();
        types::Array::bounded(len, w)?;
        for i in 0..len {
            let item = unsafe { l.get_item_unchecked(i) };
            from_pyobject(_py, &item, w)?;
        }
        Ok(())
    } else if let Ok(map) = obj.cast::<PyDict>() {
        let entries = sorted_map_entries(map)?;
        types::Map::bounded(entries.len(), w)?;
        for (key, value) in &entries {
            (&**key).encode(w)?;
            from_pyobject(_py, value, w)?;
        }
        Ok(())
    } else if let Ok(s) = obj.cast::<PyString>() {
        s.to_str()?.encode(w)?;
        Ok(())
    } else if let Ok(b) = obj.cast::<PyBytes>() {
        let bytes = b.as_bytes();
        if looks_like_cid(bytes) && ::ipld_core::cid::Cid::try_from(bytes).is_ok() {
            types::Tag(42, PrefixedCidBytes(bytes)).encode(w)?;
        } else {
            types::Bytes(bytes).encode(w)?;
        }
        Ok(())
    } else if let Ok(f) = obj.cast::<PyFloat>() {
        let v = f.value();
        if !v.is_finite() {
            return Err(anyhow!("Number out of range"));
        }
        v.encode(w)?;
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
    let mut buf = VecWriter::new();
    if let Err(e) = from_pyobject(py, data, &mut buf) {
        return Err(value_error("Failed to encode DAG-CBOR", e.to_string()));
    }
    Ok(PyBytes::new(py, buf.as_slice()))
}
