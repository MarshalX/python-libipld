//! Direct-mapped intern cache for short map keys. atproto-shape payloads
//! reuse a small vocabulary (`$type`, `did`, `cid`, `uri`, `text`, ...) per
//! record; caching the constructed `PyUnicode` + its `Py_hash_t` skips both
//! the rebuild and the rehash inside `PyDict_SetItem`.

// Cached variant: CPython with the GIL (single-threaded access to the static).
#[cfg(all(CPython, not(Py_GIL_DISABLED)))]
mod cached {
    use pyo3::{ffi, prelude::*};

    use crate::ffi::string::from_bytes;

    const CAP: usize = 2048;
    const MAX_KEY_LEN: usize = 64;

    struct Entry {
        len: u16,
        bytes: [u8; MAX_KEY_LEN],
        obj: *mut ffi::PyObject,
        hash: ffi::Py_hash_t,
    }

    impl Entry {
        const fn empty() -> Self {
            Self {
                len: 0,
                bytes: [0; MAX_KEY_LEN],
                obj: std::ptr::null_mut(),
                hash: 0,
            }
        }
    }

    static mut SLOTS: [Entry; CAP] = [const { Entry::empty() }; CAP];

    #[inline]
    fn fx_hash(bytes: &[u8]) -> usize {
        const K: u64 = 0x517c_c1b7_2722_0a95;
        let mut h: u64 = 0;
        for &b in bytes {
            h = (h.rotate_left(5) ^ b as u64).wrapping_mul(K);
        }
        h as usize
    }

    /// Returns `(strong-ref PyUnicode*, Py_hash_t)`. Caller owns one ref.
    /// Caller must hold the GIL (we are always called from a `Python<'_>`).
    #[inline]
    pub(crate) unsafe fn intern(
        py: Python<'_>,
        bytes: &[u8],
    ) -> PyResult<(*mut ffi::PyObject, ffi::Py_hash_t)> {
        if bytes.len() > MAX_KEY_LEN {
            return build(py, bytes);
        }

        let slot_idx = fx_hash(bytes) & (CAP - 1);
        // `&raw mut` is the supported path to a `static mut`; the explicit
        // re-borrow keeps the field accesses readable. Clippy's `deref_addrof`
        // suggestion would re-introduce `static_mut_refs`.
        #[allow(clippy::deref_addrof)]
        let slot = &mut *(&raw mut SLOTS[slot_idx]);

        if slot.len as usize == bytes.len()
            && !slot.obj.is_null()
            && slot.bytes[..bytes.len()] == *bytes
        {
            ffi::Py_INCREF(slot.obj);
            return Ok((slot.obj, slot.hash));
        }

        let (obj, hash) = build(py, bytes)?;
        // Evict the previous occupant before claiming the slot.
        if !slot.obj.is_null() {
            ffi::Py_DECREF(slot.obj);
        }
        // One ref for the cache, one for the caller.
        ffi::Py_INCREF(obj);
        slot.obj = obj;
        slot.hash = hash;
        slot.len = bytes.len() as u16;
        slot.bytes[..bytes.len()].copy_from_slice(bytes);
        Ok((obj, hash))
    }

    #[inline]
    unsafe fn build(
        py: Python<'_>,
        bytes: &[u8],
    ) -> PyResult<(*mut ffi::PyObject, ffi::Py_hash_t)> {
        let s = from_bytes(py, bytes)?;
        let ptr = s.as_ptr();
        let hash = ffi::PyObject_Hash(ptr);
        if hash == -1 {
            return Err(PyErr::fetch(py));
        }
        Ok((s.into_ptr(), hash))
    }
}

#[cfg(all(CPython, not(Py_GIL_DISABLED)))]
pub(crate) use cached::intern;

// Non-CPython / free-threaded fallback: no cache, just build the string and
// compute its hash inline.
#[cfg(not(all(CPython, not(Py_GIL_DISABLED))))]
pub(crate) unsafe fn intern(
    py: pyo3::Python<'_>,
    bytes: &[u8],
) -> pyo3::PyResult<(*mut pyo3::ffi::PyObject, pyo3::ffi::Py_hash_t)> {
    use pyo3::{ffi, prelude::*};

    use crate::ffi::string::from_bytes;

    let s = from_bytes(py, bytes)?;
    let ptr = s.as_ptr();
    let hash = ffi::PyObject_Hash(ptr);
    if hash == -1 {
        return Err(PyErr::fetch(py));
    }
    Ok((s.into_ptr(), hash))
}
