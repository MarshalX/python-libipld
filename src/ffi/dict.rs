use pyo3::ffi;

#[cfg(CPython)]
use anyhow::{anyhow, Result};
#[cfg(CPython)]
use pyo3::prelude::*;
#[cfg(CPython)]
use pyo3::types::PyDict;

// Empty CPython dicts already have 8 slots, so presizing below that buys
// nothing and lets us stay on the public `PyDict_New` path.
#[inline]
pub(crate) unsafe fn new_presized(len: usize) -> *mut ffi::PyObject {
    #[cfg(CPython)]
    {
        if len > 8 {
            crate::ffi::sys::_PyDict_NewPresized(len as ffi::Py_ssize_t)
        } else {
            ffi::PyDict_New()
        }
    }
    #[cfg(not(CPython))]
    {
        let _ = len;
        ffi::PyDict_New()
    }
}

// Insert by a precomputed `Py_hash_t`, skipping the rehash inside
// `PyDict_SetItem`. Steals the caller's reference to `value`.
#[cfg(CPython)]
#[inline]
pub(crate) unsafe fn set_item_known_hash(
    py: Python<'_>,
    dict: &Bound<'_, PyDict>,
    key: &Bound<'_, PyAny>,
    value: Py<PyAny>,
    hash: ffi::Py_hash_t,
) -> Result<()> {
    let value_ptr = value.into_ptr();
    let rc =
        crate::ffi::sys::_PyDict_SetItem_KnownHash(dict.as_ptr(), key.as_ptr(), value_ptr, hash);
    ffi::Py_DECREF(value_ptr);
    if rc != 0 {
        return Err(anyhow!(PyErr::fetch(py)));
    }
    Ok(())
}
