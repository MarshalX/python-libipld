use pyo3::prelude::*;
use pyo3::sync::critical_section::with_critical_section;
use pyo3::types::*;

use crate::error::value_error;

/// Run `f` on a byte view of a `bytes`, `bytearray`, or `str` (UTF-8) object.
/// A `bytearray` is mutable, so on the free-threaded build its view is only
/// valid while its critical section is held; `f` must not call into Python.
pub(crate) fn with_bytes<R>(
    obj: &Bound<'_, PyAny>,
    f: impl FnOnce(&[u8]) -> PyResult<R>,
) -> PyResult<R> {
    if let Ok(b) = obj.cast::<PyBytes>() {
        f(b.as_bytes())
    } else if let Ok(ba) = obj.cast::<PyByteArray>() {
        with_critical_section(ba, || f(unsafe { ba.as_bytes() }))
    } else if let Ok(s) = obj.cast::<PyString>() {
        f(s.to_str()?.as_bytes())
    } else {
        Err(value_error(
            "Failed to encode multibase",
            "Unsupported data type".to_string(),
        ))
    }
}
