use pyo3::prelude::*;

use crate::convert::with_bytes;
use crate::error::value_error;

#[pyfunction]
pub fn encode_multibase(code: char, data: &Bound<PyAny>) -> PyResult<String> {
    with_bytes(data, |data_bytes| {
        let base = ::cid::multibase::Base::from_code(code);
        if let Ok(base) = base {
            Ok(::cid::multibase::encode(base, data_bytes))
        } else {
            Err(value_error(
                "Failed to encode multibase",
                base.unwrap_err().to_string(),
            ))
        }
    })
}
