use pyo3::prelude::*;
use pyo3::types::*;

use crate::cid::extract_cid;

#[pyfunction]
pub fn encode_cid<'py>(py: Python<'py>, data: &Bound<PyAny>) -> PyResult<Bound<'py, PyString>> {
    Ok(PyString::new(py, extract_cid(data)?.to_string().as_str()))
}
