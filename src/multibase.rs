//! Multibase string codec (encode/decode of self-describing base encodings).

pub(crate) mod de;
pub(crate) mod ser;

pub use de::decode_multibase;
pub use ser::encode_multibase;
