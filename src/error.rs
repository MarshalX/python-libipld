use pyo3::PyErr;

/// Build a `ValueError` of the form `"{msg}. {detail}"`.
pub(crate) fn value_error(msg: &str, detail: String) -> PyErr {
    PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}. {}", msg, detail))
}
