use pyo3::prelude::*;

use crate::convert::extract_bytes;
use crate::error::value_error;

#[pyfunction]
pub fn encode_multibase(code: char, data: &Bound<PyAny>) -> PyResult<String> {
    let data_bytes = extract_bytes(data)?;
    let base = ::ipld_core::cid::multibase::Base::from_code(code);
    if let Ok(base) = base {
        Ok(::ipld_core::cid::multibase::encode(base, data_bytes))
    } else {
        Err(value_error(
            "Failed to encode multibase",
            base.unwrap_err().to_string(),
        ))
    }
}
