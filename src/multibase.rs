//! Multibase string codec (encode/decode of self-describing base encodings).

pub(crate) mod de;
pub(crate) mod ser;

pub(crate) use de::decode_multibase;
pub(crate) use ser::encode_multibase;
