//! Private CPython symbols; not provided by pyo3-ffi and CPython-only.

use pyo3::ffi;

include!(concat!(env!("OUT_DIR"), "/extern_libpython.rs"));

extern_libpython! {
    // x86 Windows import names drop one leading `_`.
    #[cfg_attr(all(windows, target_arch = "x86"), link_name = "__PyDict_NewPresized")]
    pub(crate) fn _PyDict_NewPresized(minused: ffi::Py_ssize_t) -> *mut ffi::PyObject;
}
