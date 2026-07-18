use std::cell::Cell;
use std::convert::Infallible;

use cbor4ii::core::enc;

// Retaining bigger buffers would pin worst-case memory per thread forever.
const MAX_POOLED_CAPACITY: usize = 1 << 20;

thread_local! {
    static POOL: Cell<Vec<u8>> = const { Cell::new(Vec::new()) };
}

// `enc::Write` over a `Vec<u8>` recycled through a thread-local pool: encoding
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

impl enc::Write for VecWriter {
    type Error = Infallible;

    #[inline]
    fn push(&mut self, input: &[u8]) -> Result<(), Self::Error> {
        self.0.extend_from_slice(input);
        Ok(())
    }
}
