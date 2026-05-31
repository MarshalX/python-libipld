use anyhow::{anyhow, Result};
use cbor4ii::core::dec;

// Based on cbor4ii/src/utils.rs.
/// An in-memory reader.
pub(crate) struct SliceReader<'a> {
    pub(crate) buf: &'a [u8],
}

impl SliceReader<'_> {
    pub(crate) fn new(buf: &[u8]) -> SliceReader<'_> {
        SliceReader { buf }
    }
}

impl<'de> dec::Read<'de> for SliceReader<'de> {
    type Error = core::convert::Infallible;

    #[inline]
    fn fill<'b>(&'b mut self, want: usize) -> Result<dec::Reference<'de, 'b>, Self::Error> {
        let len = core::cmp::min(self.buf.len(), want);
        Ok(dec::Reference::Long(&self.buf[..len]))
    }

    #[inline]
    fn advance(&mut self, n: usize) {
        let len = core::cmp::min(self.buf.len(), n);
        self.buf = &self.buf[len..];
    }
}

// Based on cbor4ii code.
pub(crate) fn peek_one<'de, R: dec::Read<'de>>(r: &mut R) -> Result<u8>
where
    R::Error: Send + Sync,
{
    r.fill(1)?
        .as_ref()
        .first()
        .copied()
        .ok_or_else(|| anyhow!("end of data"))
}
