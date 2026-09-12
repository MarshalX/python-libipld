use anyhow::anyhow;
use pyo3::{PyErr, Python};

/// Build a `ValueError` of the form `"{msg}. {detail}"`.
pub(crate) fn value_error(msg: &str, detail: String) -> PyErr {
    PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}. {}", msg, detail))
}

pub(crate) fn pending_or_value_error(py: Python, msg: &str, e: anyhow::Error) -> PyErr {
    let err = value_error(msg, e.to_string());
    if let Some(py_err) = PyErr::take(py) {
        py_err.set_cause(py, Some(err));
        py_err
    } else {
        err
    }
}

pub(crate) fn recursion_error(py: Python, message: &str) -> anyhow::Error {
    PyErr::new::<pyo3::exceptions::PyRecursionError, _>(message.to_string()).restore(py);
    anyhow!("Maximum recursion depth exceeded")
}
