use pyo3::ffi;

// Snapshot `sys.getrecursionlimit()` once per top-level decode call and pass
// it through. Calling `ffi::Py_GetRecursionLimit()` from the hot path costs
// ~5–10 ns per recursive step, which dominates on scalar-dense payloads
// (canada makes 111k+ recursive calls, one per float).
#[inline]
pub(crate) fn current_recursion_limit() -> usize {
    unsafe { ffi::Py_GetRecursionLimit() as usize }
}
