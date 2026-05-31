//! Unsafe CPython interop layer.
//!
//! Everything here is `#[cfg]`-gated against the interpreter (CPython vs other,
//! Python version, free-threaded vs GIL) and reaches into CPython internals or
//! object layouts that the public `pyo3` API does not expose. The domain
//! modules call into these fast paths; the danger stays quarantined here.

pub(crate) mod dict;
pub(crate) mod int;
pub(crate) mod key_cache;
pub(crate) mod recursion;
pub(crate) mod string;

// Private CPython symbols only resolve on a real CPython build.
#[cfg(CPython)]
pub(crate) mod sys;
