//! Copy and compare for short byte runs, where a `memcpy`/`memcmp` call costs
//! more than the work itself.

/// `copy_nonoverlapping` that handles up to 32 bytes inline with two
/// overlapping fixed-width moves.
///
/// # Safety
/// `src` must be readable and `dst` writable for `len` bytes, non-overlapping.
#[inline(always)]
pub(crate) unsafe fn copy_short(src: *const u8, dst: *mut u8, len: usize) {
    macro_rules! overlapping {
        ($t:ty) => {{
            let tail = len - std::mem::size_of::<$t>();
            let first = src.cast::<$t>().read_unaligned();
            let last = src.add(tail).cast::<$t>().read_unaligned();
            dst.cast::<$t>().write_unaligned(first);
            dst.add(tail).cast::<$t>().write_unaligned(last);
        }};
    }
    // An `if` chain keeps this branch order; a `match` measured 3-6% slower.
    if len > 32 {
        std::ptr::copy_nonoverlapping(src, dst, len);
    } else if len >= 16 {
        overlapping!(u128)
    } else if len >= 8 {
        overlapping!(u64)
    } else if len >= 4 {
        overlapping!(u32)
    } else if len >= 2 {
        overlapping!(u16)
    } else if len == 1 {
        *dst = *src;
    }
}

/// Equality of two same-length byte runs, compared a word at a time.
#[cfg(all(CPython, not(Py_GIL_DISABLED)))]
#[inline(always)]
pub(crate) fn eq_short(a: &[u8], b: &[u8]) -> bool {
    debug_assert_eq!(a.len(), b.len());
    if a.len() >= 8 {
        a.chunks_exact(8)
            .zip(b.chunks_exact(8))
            .all(|(x, y)| x == y)
            && a.last_chunk::<8>() == b.last_chunk::<8>()
    } else if a.len() >= 4 {
        a.first_chunk::<4>() == b.first_chunk::<4>() && a.last_chunk::<4>() == b.last_chunk::<4>()
    } else {
        a.iter().zip(b).all(|(x, y)| x == y)
    }
}
