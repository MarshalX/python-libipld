//! Direct-mapped intern cache for short map keys. atproto-shape payloads
//! reuse a small vocabulary (`$type`, `did`, `cid`, `uri`, `text`, ...) per
//! record; caching the constructed `PyUnicode`, hash already computed and
//! stored inside it, skips both the rebuild and the rehash on dict insert.

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
    }

    impl Entry {
        const fn empty() -> Self {
            Self {
                len: 0,
                bytes: [0; MAX_KEY_LEN],
                obj: std::ptr::null_mut(),
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

    /// Returns a strong-ref `PyUnicode*`; the caller owns one ref.
    /// Caller must hold the GIL (we are always called from a `Python<'_>`).
    #[inline]
    pub(crate) unsafe fn intern(py: Python<'_>, bytes: &[u8]) -> PyResult<*mut ffi::PyObject> {
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
            return Ok(slot.obj);
        }

        let obj = build(py, bytes)?;
        // Evict the previous occupant before claiming the slot.
        if !slot.obj.is_null() {
            ffi::Py_DECREF(slot.obj);
        }
        // One ref for the cache, one for the caller.
        ffi::Py_INCREF(obj);
        slot.obj = obj;
        slot.len = bytes.len() as u16;
        slot.bytes[..bytes.len()].copy_from_slice(bytes);
        Ok(obj)
    }

    // Hashing up front stores the hash inside the `str`, so every later dict
    // insert of this key reads it instead of rehashing.
    #[inline]
    unsafe fn build(py: Python<'_>, bytes: &[u8]) -> PyResult<*mut ffi::PyObject> {
        let s = from_bytes(py, bytes)?;
        if ffi::PyObject_Hash(s.as_ptr()) == -1 {
            return Err(PyErr::fetch(py));
        }
        Ok(s.into_ptr())
    }
}

#[cfg(all(CPython, not(Py_GIL_DISABLED)))]
pub(crate) use cached::intern;

// Non-CPython / free-threaded fallback: no cache, just build the string.
#[cfg(not(all(CPython, not(Py_GIL_DISABLED))))]
pub(crate) unsafe fn intern(
    py: pyo3::Python<'_>,
    bytes: &[u8],
) -> pyo3::PyResult<*mut pyo3::ffi::PyObject> {
    use crate::ffi::string::from_bytes;

    Ok(from_bytes(py, bytes)?.into_ptr())
}
