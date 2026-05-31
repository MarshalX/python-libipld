use pyo3::prelude::*;
use pyo3::types::PyString;

#[cfg(CPython)]
use pyo3::ffi;

// `PyUnicode_DecodeUTF8` runs a state machine even on pure-ASCII input. Skip
// it by allocating a compact-ASCII `PyUnicode` and memcpying into its inline
// buffer; non-ASCII falls through to the standard decoder.
#[cfg(CPython)]
#[inline]
pub(crate) fn from_bytes<'py>(py: Python<'py>, bytes: &[u8]) -> PyResult<Bound<'py, PyString>> {
    if !bytes.is_ascii() {
        return PyString::from_bytes(py, bytes);
    }

    unsafe {
        let obj = ffi::PyUnicode_New(bytes.len() as ffi::Py_ssize_t, 127);
        if obj.is_null() {
            return Err(PyErr::fetch(py));
        }

        let data = obj.cast::<ffi::PyASCIIObject>().offset(1).cast::<u8>();
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), data, bytes.len());
        *data.add(bytes.len()) = 0;

        Ok(Bound::from_owned_ptr(py, obj).cast_into_unchecked::<PyString>())
    }
}

#[cfg(not(CPython))]
#[inline]
pub(crate) fn from_bytes<'py>(py: Python<'py>, bytes: &[u8]) -> PyResult<Bound<'py, PyString>> {
    PyString::from_bytes(py, bytes)
}
