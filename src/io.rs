//! IO primitives shared by the codecs: an in-memory reader, a `Vec`-backed
//! writer, and the LEB128 varint reader used by the CAR container format.

pub(crate) mod leb128;
pub(crate) mod reader;
pub(crate) mod writer;

pub(crate) use reader::{peek_one, SliceReader};
pub(crate) use writer::VecWriter;
