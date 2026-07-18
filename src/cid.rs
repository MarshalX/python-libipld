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

    pub(crate) fn extract_cid(data: &Bound<PyAny>) -> PyResult<::ipld_core::cid::Cid> {
    let cid = if let Ok(s) = data.cast::<PyString>() {
        ::ipld_core::cid::Cid::try_from(s.to_str()?)
    } else {
        ::ipld_core::cid::Cid::try_from(extract_bytes(data)?)
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
