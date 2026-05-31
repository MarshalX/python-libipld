use pyo3::prelude::*;
use pyo3::types::*;

use crate::cid::extract_cid;

fn hash_to_pydict<'py>(py: Python<'py>, cid: &::cid::Cid) -> PyResult<Bound<'py, PyDict>> {
    let hash = cid.hash();
    let dict_obj = PyDict::new(py);

    dict_obj.set_item("code", hash.code())?;
    dict_obj.set_item("size", hash.size())?;
    dict_obj.set_item("digest", PyBytes::new(py, hash.digest()))?;

    Ok(dict_obj)
}

fn to_pydict<'py>(py: Python<'py>, cid: &::cid::Cid) -> PyResult<Bound<'py, PyDict>> {
    let dict_obj = PyDict::new(py);

    dict_obj.set_item("version", cid.version() as u64)?;
    dict_obj.set_item("codec", cid.codec())?;
    dict_obj.set_item("hash", hash_to_pydict(py, cid)?)?;
    Ok(dict_obj)
}

#[pyfunction]
pub fn decode_cid<'py>(py: Python<'py>, data: &Bound<PyAny>) -> PyResult<Bound<'py, PyDict>> {
    to_pydict(py, &extract_cid(data)?)
}
