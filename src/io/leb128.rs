use anyhow::{anyhow, Result};
use cbor4ii::core::dec;

use crate::io::reader::peek_one;

#[inline]
pub(crate) fn read_u64<'de, R: dec::Read<'de>>(r: &mut R) -> Result<u64>
where
    R::Error: Send + Sync,
{
    let mut result: u64 = 0;
    let mut shift = 0;

    loop {
        let byte =
            peek_one(r).map_err(|_| anyhow!("Unexpected EOF while reading ULEB128 number."))?;
        r.advance(1);

        if shift == 63 && byte != 0x00 && byte != 0x01 {
            // consume remaining continuation bytes so reader stays in sync
            let mut b = byte;
            while b & 0x80 != 0 {
                b = peek_one(r).map_err(|_| {
                    anyhow!("Unexpected EOF while skipping overflowing ULEB128 number.")
                })?;
                r.advance(1);
            }
            return Err(anyhow!("ULEB128 overflow"));
        }

        let low_bits = (byte & !0x80) as u64;
        result |= low_bits << shift;

        if byte & 0x80 == 0 {
            return Ok(result);
        }

        shift += 7;
    }
}
