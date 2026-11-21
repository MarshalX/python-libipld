use pyo3::prelude::*;
use pyo3::types::*;

use crate::cid::{extract_cid, Cid};

#[pyfunction]
pub fn encode_cid<'py>(py: Python<'py>, data: &Bound<PyAny>) -> PyResult<Bound<'py, PyString>> {
    if let Ok(cid) = data.cast::<Cid>() {
        return cid.get().string(py).cloned();
    }
    Ok(PyString::new(py, extract_cid(data)?.to_string().as_str()))
}
