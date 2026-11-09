use std::cell::Cell;

use crate::io::short::copy_short;

// Retaining bigger buffers would pin worst-case memory per thread forever.
const MAX_POOLED_CAPACITY: usize = 1 << 20;

thread_local! {
    static POOL: Cell<Vec<u8>> = const { Cell::new(Vec::new()) };
}

// CBOR writer over a `Vec<u8>` recycled through a thread-local pool: encoding
// many small records reuses one grown allocation instead of re-growing from
// zero on every call. `Cell::take` leaves an empty `Vec` behind, so a
// re-entrant encode just falls back to a fresh buffer.
pub(crate) struct VecWriter(Vec<u8>);

impl VecWriter {
    #[inline]
    pub(crate) fn new() -> Self {
        let mut buf = POOL.take();
        buf.clear();
        VecWriter(buf)
    }

    #[inline]
    pub(crate) fn as_slice(&self) -> &[u8] {
        &self.0
    }
}

impl Drop for VecWriter {
    fn drop(&mut self) {
        let buf = std::mem::take(&mut self.0);
        if buf.capacity() <= MAX_POOLED_CAPACITY {
            POOL.set(buf);
        }
    }
}

// Major types, already shifted into the top 3 bits of the header byte.
pub(crate) const UNSIGNED: u8 = 0x00;
pub(crate) const NEGATIVE: u8 = 0x20;
pub(crate) const BYTES: u8 = 0x40;
pub(crate) const STRING: u8 = 0x60;
pub(crate) const ARRAY: u8 = 0x80;
pub(crate) const MAP: u8 = 0xa0;
pub(crate) const TAG: u8 = 0xc0;

const FALSE: u8 = 0xf4;
const TRUE: u8 = 0xf5;
const NULL: u8 = 0xf6;
const F64: u8 = 0xfb;

// Longest header: the type byte plus an 8-byte argument.
const MAX_HEAD: usize = 9;

// Writes the shortest header for `major` with argument `n` and returns its
// length. `p` must be writable for `MAX_HEAD` bytes.
#[inline(always)]
unsafe fn put_head(p: *mut u8, major: u8, n: u64) -> usize {
    macro_rules! put {
        ($info:literal, $t:ty) => {{
            *p = major | $info;
            p.add(1).cast::<$t>().write_unaligned((n as $t).to_be());
            1 + std::mem::size_of::<$t>()
        }};
    }
    if n < 24 {
        *p = major | n as u8;
        1
    } else if n <= u8::MAX as u64 {
        put!(24, u8)
    } else if n <= u16::MAX as u64 {
        put!(25, u16)
    } else if n <= u32::MAX as u64 {
        put!(26, u32)
    } else {
        put!(27, u64)
    }
}

impl VecWriter {
    /// Header of `major` with argument `n`: an integer value, a container
    /// length or a tag number.
    #[inline]
    pub(crate) fn head(&mut self, major: u8, n: u64) {
        self.0.reserve(MAX_HEAD);
        unsafe {
            let len = self.0.len();
            let head = put_head(self.0.as_mut_ptr().add(len), major, n);
            self.0.set_len(len + head);
        }
    }

    /// Length header followed by `data`: a whole byte or text string.
    #[inline]
    pub(crate) fn segment(&mut self, major: u8, data: &[u8]) {
        self.0.reserve(MAX_HEAD + data.len());
        unsafe {
            let len = self.0.len();
            let p = self.0.as_mut_ptr().add(len);
            let head = put_head(p, major, data.len() as u64);
            copy_short(data.as_ptr(), p.add(head), data.len());
            self.0.set_len(len + head + data.len());
        }
    }

    #[inline]
    pub(crate) fn raw(&mut self, data: &[u8]) {
        self.0.extend_from_slice(data);
    }

    #[inline]
    pub(crate) fn bool(&mut self, v: bool) {
        self.0.push(if v { TRUE } else { FALSE });
    }

    #[inline]
    pub(crate) fn null(&mut self) {
        self.0.push(NULL);
    }

    #[inline]
    pub(crate) fn f64(&mut self, v: f64) {
        let mut buf = [F64; 9];
        buf[1..].copy_from_slice(&v.to_be_bytes());
        self.0.extend_from_slice(&buf);
    }
}
