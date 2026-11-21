//! CID (Content IDentifier) codec, the `CID` type, and the shared CID helpers.

pub(crate) mod de;
pub(crate) mod object;
pub(crate) mod ser;

pub use de::decode_cid;
pub use object::{Cid, Multihash};
pub use ser::encode_cid;

use pyo3::prelude::*;
use pyo3::types::*;

use crate::convert::with_bytes;
use crate::error::value_error;

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

/// Whether `bytes` is exactly one CID, as `::cid::Cid::to_bytes` writes it.
#[inline]
pub(crate) fn is_canonical_cid(bytes: &[u8]) -> bool {
    if let [1, ..=0x7f, ..=0x7f, size @ ..=64, digest @ ..] = bytes {
        return digest.len() == *size as usize;
    }
    if let [0x12, 0x20, digest @ ..] = bytes {
        return digest.len() == 32;
    }
    let [1, rest @ ..] = bytes else {
        return false;
    };

    let mut pos = 0;
    let mut hash_size = 0;
    // codec, multihash code, digest size
    for _ in 0..3 {
        let start = pos;
        let Some(value) = read_varint(rest, &mut pos) else {
            return false;
        };
        if pos - start == 10 && rest[pos - 1] > 1 {
            return false;
        }
        hash_size = value;
    }
    hash_size <= 64 && rest.len() - pos == hash_size as usize
}

#[cold]
pub(crate) fn cid_error(bytes: &[u8]) -> &'static str {
    match parse_cid_prefix(bytes) {
        None => "Invalid CID",
        Some((end, _)) if end < bytes.len() => "Invalid CID: bytes after the CID",
        Some(_) => "Invalid CID: varint over 64 bits",
    }
}

pub(crate) fn extract_cid(data: &Bound<PyAny>) -> PyResult<::cid::Cid> {
    if let Ok(cid) = data.cast::<Cid>() {
        parse_cid(cid.get().as_bytes())
    } else if let Ok(s) = data.cast::<PyString>() {
        parse_cid(s.to_str()?)
    } else {
        with_bytes(data, |b| parse_cid(b))
    }
}

#[inline]
fn parse_cid<T>(data: T) -> PyResult<::cid::Cid>
where
    ::cid::Cid: TryFrom<T, Error = ::cid::Error>,
{
    ::cid::Cid::try_from(data).map_err(|e| value_error("Failed to decode CID", e.to_string()))
}
