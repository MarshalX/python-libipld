//! DAG-CBOR codec: decode (`de`) and encode (`ser`) of the IPLD data model
//! to and from native Python objects.

pub(crate) mod de;
pub(crate) mod ser;

pub(crate) use de::{decode_dag_cbor, decode_dag_cbor_multi};
pub(crate) use ser::encode_dag_cbor;
