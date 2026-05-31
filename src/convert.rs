use pyo3::prelude::*;
use pyo3::types::*;

use crate::error::value_error;

/// Borrow a byte view from a `bytes`, `bytearray`, or `str` (UTF-8) object.
pub(crate) fn extract_bytes<'py>(obj: &'py Bound<'py, PyAny>) -> PyResult<&'py [u8]> {
    if let Ok(b) = obj.cast::<PyBytes>() {
        Ok(b.as_bytes())
    } else if let Ok(ba) = obj.cast::<PyByteArray>() {
        Ok(unsafe { ba.as_bytes() })
    } else if let Ok(s) = obj.cast::<PyString>() {
        Ok(s.to_str()?.as_bytes())
    } else {
        Err(value_error(
            "Failed to encode multibase",
            "Unsupported data type".to_string(),
        ))
    }
}
