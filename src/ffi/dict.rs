use anyhow::{anyhow, Result};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3::{ffi, Borrowed};

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

// Insert a `str` key whose hash is already cached inside it, so
// `PyDict_SetItem` never rehashes. Steals the caller's reference to `value`.
#[inline]
pub(crate) unsafe fn set_item(
    py: Python<'_>,
    dict: &Bound<'_, PyDict>,
    key: Borrowed<'_, '_, PyAny>,
    value: Py<PyAny>,
) -> Result<()> {
    let value_ptr = value.into_ptr();
    let rc = ffi::PyDict_SetItem(dict.as_ptr(), key.as_ptr(), value_ptr);
    ffi::Py_DECREF(value_ptr);
    if rc != 0 {
        return Err(anyhow!(PyErr::fetch(py)));
    }
    Ok(())
}
