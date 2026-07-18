use pyo3::prelude::*;
use pyo3::types::*;

use crate::error::value_error;

#[pyfunction]
pub fn decode_multibase<'py>(py: Python<'py>, data: &str) -> PyResult<(char, Bound<'py, PyBytes>)> {
    let base = ::ipld_core::cid::multibase::decode(data);
    if let Ok((base, data)) = base {
        Ok((base.code(), PyBytes::new(py, &data)))
    } else {
        Err(value_error(
            "Failed to decode multibase",
            base.unwrap_err().to_string(),
        ))
    }
}
