#[cfg(all(CPython, Py_3_12))]
use pyo3::ffi;

// CPython 3.12+ PyLongObject layout: `PyObject_HEAD; uintptr_t lv_tag; digit ob_digit[]`.
// `lv_tag` packs the sign in the low 3 bits (0=positive, 1=zero, 2=negative) and the
// digit count in the upper bits. Default builds use 30-bit digits (uint32_t).
//
// Returns `(abs_val, neg)` for ints that fit in two digits, or `None` when the
// caller should fall back to the generic `i128` extraction path.
#[cfg(all(CPython, Py_3_12))]
#[inline]
pub(crate) unsafe fn pylong_parts(obj: *mut ffi::PyObject) -> Option<(u64, bool)> {
    const NON_SIZE_BITS: u32 = 3;
    const SIGN_MASK: usize = 3;
    const SIGN_NEGATIVE: usize = 2;
    const PYLONG_DIGIT_BITS: u32 = 30;

    let lv_tag_ptr = (obj as *const u8).add(std::mem::size_of::<ffi::PyObject>()) as *const usize;
    let lv_tag = *lv_tag_ptr;
    let ndigits = lv_tag >> NON_SIZE_BITS;
    let neg = (lv_tag & SIGN_MASK) == SIGN_NEGATIVE;

    let ob_digit = lv_tag_ptr.add(1) as *const u32;
    let abs_val: u64 = match ndigits {
        0 => return Some((0, false)),
        1 => *ob_digit as u64,
        2 => (*ob_digit as u64) | ((*ob_digit.add(1) as u64) << PYLONG_DIGIT_BITS),
        _ => return None,
    };
    Some((abs_val, neg))
}
