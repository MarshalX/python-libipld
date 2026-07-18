//! CID (Content IDentifier) codec plus the shared CID helpers used across
//! codecs: extraction from arbitrary Python objects and the O(1) shape check.

pub(crate) mod de;
pub(crate) mod ser;

pub(crate) use de::decode_cid;
pub(crate) use ser::encode_cid;

use pyo3::prelude::*;
use pyo3::types::*;

use crate::convert::extract_bytes;
use crate::error::value_error;

// `Cid::try_from` parses two varints + a multihash on every call; this O(1)
// shape check rejects payloads that can't be a CID without paying for it.
// CIDv1 starts with `0x01`; CIDv0 is exactly 34 bytes starting `0x12 0x20`.
#[inline]
pub(crate) fn looks_like_cid(bytes: &[u8]) -> bool {
    if bytes.len() < 4 {
        return false;
    }
    if bytes[0] == 0x01 {
        return true;
    }
    bytes.len() == 34 && bytes[0] == 0x12 && bytes[1] == 0x20
}

// Minimal-encoding unsigned varint, as `unsigned_varint::decode::u64`:
// ≤10 bytes, last byte of a multi-byte varint must be non-zero.
#[inline]
fn read_varint(bytes: &[u8], pos: &mut usize) -> Option<u64> {
    let mut n: u64 = 0;
    for i in 0..10 {
        let &b = bytes.get(*pos + i)?;
        n |= ((b & 0x7f) as u64) << (i * 7);
        if b & 0x80 == 0 {
            if b == 0 && i > 0 {
                return None;
            }
            *pos += i + 1;
            return Some(n);
        }
    }
    None
}

/// Structural check that `bytes` starts with a valid binary CID; returns
/// `(consumed, codec)`. Accepts exactly what `::cid::Cid::try_from` accepts
/// (trailing bytes are the caller's concern) but skips the `Multihash`
/// construction and its 64-byte digest copy.
#[inline]
pub(crate) fn parse_cid_prefix(bytes: &[u8]) -> Option<(usize, u64)> {
    let mut pos = 0;
    let version = read_varint(bytes, &mut pos)?;
    let codec = read_varint(bytes, &mut pos)?;

    // CIDv0: `0x12 0x20` + 32-byte sha2-256 digest; codec is implicitly dag-pb
    if (version, codec) == (0x12, 0x20) {
        let end = pos + 32;
        return (bytes.len() >= end).then_some((end, 0x70));
    }
    if version != 1 {
        return None;
    }

    read_varint(bytes, &mut pos)?; // multihash code
    let hash_size = read_varint(bytes, &mut pos)?;
    if hash_size > 64 {
        return None;
    }
    let end = pos + hash_size as usize;
    (bytes.len() >= end).then_some((end, codec))
}

pub(crate) fn extract_cid(data: &Bound<PyAny>) -> PyResult<::cid::Cid> {
    let cid = if let Ok(s) = data.cast::<PyString>() {
        ::cid::Cid::try_from(s.to_str()?)
    } else {
        ::cid::Cid::try_from(extract_bytes(data)?)
    };

    if let Ok(cid) = cid {
        Ok(cid)
    } else {
        Err(value_error(
            "Failed to decode CID",
            cid.unwrap_err().to_string(),
        ))
    }
}
