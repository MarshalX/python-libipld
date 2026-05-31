use std::convert::Infallible;

use cbor4ii::core::enc;

// `enc::Write` over a raw `Vec<u8>`: no syscalls behind it, so a `BufWriter`
// wrapper would just add a memcpy per push for no benefit.
pub(crate) struct VecWriter(Vec<u8>);

impl VecWriter {
    #[inline]
    pub(crate) fn new() -> Self {
        VecWriter(Vec::new())
    }

    #[inline]
    pub(crate) fn as_slice(&self) -> &[u8] {
        &self.0
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
