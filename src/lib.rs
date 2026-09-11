use pyo3::prelude::*;

mod car;
mod cid;
mod convert;
mod dag_cbor;
mod error;
mod ffi;
mod io;
mod multibase;

pub use car::decode_car;
pub use cid::{decode_cid, encode_cid};
pub use dag_cbor::{decode_dag_cbor, decode_dag_cbor_multi, encode_dag_cbor};
pub use multibase::{decode_multibase, encode_multibase};

#[pymodule]
#[pyo3(name = "_libipld")]
fn libipld(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(cid::decode_cid, m)?)?;
    m.add_function(wrap_pyfunction!(cid::encode_cid, m)?)?;

    m.add_function(wrap_pyfunction!(car::decode_car, m)?)?;

    m.add_function(wrap_pyfunction!(dag_cbor::decode_dag_cbor, m)?)?;
    m.add_function(wrap_pyfunction!(dag_cbor::decode_dag_cbor_multi, m)?)?;
    m.add_function(wrap_pyfunction!(dag_cbor::encode_dag_cbor, m)?)?;

    m.add_function(wrap_pyfunction!(multibase::decode_multibase, m)?)?;
    m.add_function(wrap_pyfunction!(multibase::encode_multibase, m)?)?;

    Ok(())
}
