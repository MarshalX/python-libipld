//! Private CPython symbols; not provided by pyo3-ffi and CPython-only.

use pyo3::ffi;

extern "C" {
    pub(crate) fn _PyDict_NewPresized(minused: ffi::Py_ssize_t) -> *mut ffi::PyObject;
    pub(crate) fn _PyDict_SetItem_KnownHash(
        op: *mut ffi::PyObject,
        key: *mut ffi::PyObject,
        value: *mut ffi::PyObject,
        hash: ffi::Py_hash_t,
    ) -> std::os::raw::c_int;
}
