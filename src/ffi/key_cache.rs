//! Two-way intern cache for short map keys. atproto-shape payloads
//! reuse a small vocabulary (`$type`, `did`, `cid`, `uri`, `text`, ...) per
//! record; caching the constructed `PyUnicode`, hash already computed and
//! stored inside it, skips both the rebuild and the rehash on dict insert.

// Cached variant: CPython with the GIL (single-threaded access to the static).
#[cfg(all(CPython, not(Py_GIL_DISABLED)))]
mod cached {
    use pyo3::{ffi, prelude::*};

    use crate::ffi::string::from_bytes;
    use crate::io::short::eq_short;

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

    // Hashes the length plus the first and last (up to) 8 bytes. Longer keys
    // differing only in the middle share a slot: more evictions, same result.
    #[inline]
    fn slot_index(bytes: &[u8]) -> usize {
        const K: u64 = 0x9e37_79b9_7f4a_7c15;
        let (head, tail) = match (bytes.first_chunk::<8>(), bytes.last_chunk::<8>()) {
            (Some(head), Some(tail)) => (u64::from_le_bytes(*head), u64::from_le_bytes(*tail)),
            _ => match (bytes.first_chunk::<4>(), bytes.last_chunk::<4>()) {
                (Some(head), Some(tail)) => (
                    u32::from_le_bytes(*head) as u64,
                    u32::from_le_bytes(*tail) as u64,
                ),
                _ => (bytes.iter().fold(0, |acc, &b| (acc << 8) | b as u64), 0),
            },
        };
        let len = bytes.len() as u64;
        let hash = ((head ^ len.wrapping_mul(K)).wrapping_mul(K) ^ tail).wrapping_mul(K);
        (hash >> (64 - CAP.trailing_zeros())) as usize
    }

    /// Returns a strong-ref `PyUnicode*`; the caller owns one ref.
    /// Caller must hold the GIL (we are always called from a `Python<'_>`).
    #[inline]
    pub(crate) unsafe fn intern(py: Python<'_>, bytes: &[u8]) -> PyResult<*mut ffi::PyObject> {
        if bytes.len() > MAX_KEY_LEN {
            return build(py, bytes);
        }

        // A key lives in one of two adjacent slots, so two hot keys that hash
        // alike don't keep evicting each other.
        let idx = slot_index(bytes);
        let first = &raw mut SLOTS[idx];
        let second = &raw mut SLOTS[idx ^ 1];
        for slot in [first, second] {
            let slot = &*slot;
            if !slot.obj.is_null()
                && slot.len as usize == bytes.len()
                && eq_short(&slot.bytes[..bytes.len()], bytes)
            {
                ffi::Py_INCREF(slot.obj);
                return Ok(slot.obj);
            }
        }

        let obj = build(py, bytes)?;
        // Prefer an empty slot; otherwise evict the first.
        let slot = if !(*first).obj.is_null() && (*second).obj.is_null() {
            &mut *second
        } else {
            &mut *first
        };
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
