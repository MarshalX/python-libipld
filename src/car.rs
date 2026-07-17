//! CAR (Content Addressable aRchive) v1 container decoding. Encoding is not
//! implemented yet; when it lands this becomes `car/{de,ser}.rs`.

use cbor4ii::core::dec::Read;
use pyo3::prelude::*;
use pyo3::types::*;

use crate::cid::parse_cid_prefix;
use crate::dag_cbor::de::to_pyobject;
use crate::error::value_error;
use crate::ffi::recursion::current_recursion_limit;
use crate::io::leb128::read_u64;
use crate::io::SliceReader;

#[pyfunction]
pub fn decode_car<'py>(py: Python<'py>, data: &[u8]) -> PyResult<(Py<PyAny>, Bound<'py, PyDict>)> {
    let buf = &mut SliceReader::new(data);
    let max_depth = current_recursion_limit();

    if read_u64(buf).is_err() {
        return Err(value_error(
            "Failed to read CAR header",
            "Invalid uvarint".to_string(),
        ));
    }
    let Ok(header_obj) = to_pyobject(py, buf, 0, max_depth) else {
        return Err(value_error(
            "Failed to read CAR header",
            "Invalid DAG-CBOR".to_string(),
        ));
    };

    let header = header_obj.cast_bound::<PyDict>(py)?;

    let Some(version) = header.get_item("version")? else {
        return Err(value_error(
            "Failed to read CAR header",
            "Version is None".to_string(),
        ));
    };
    if version.cast::<PyInt>()?.extract::<u64>()? != 1 {
        return Err(value_error(
            "Failed to read CAR header",
            "Unsupported version. Version must be 1".to_string(),
        ));
    }

    let Some(roots) = header.get_item("roots")? else {
        return Err(value_error(
            "Failed to read CAR header",
            "Roots is None".to_string(),
        ));
    };
    if roots.cast::<PyList>()?.len() == 0 {
        return Err(value_error(
            "Failed to read CAR header",
            "Roots is empty. Must be at least one".to_string(),
        ));
    }

    // FIXME (MarshalX): we are not verifying if the roots are valid CIDs

    let parsed_blocks = PyDict::new(py);

    loop {
        if read_u64(buf).is_err() {
            // FIXME (MarshalX): we are not raising an error here because of possible EOF
            break;
        }

        let cid_bytes_before = buf.buf;
        let Some((consumed, codec)) = parse_cid_prefix(cid_bytes_before) else {
            return Err(value_error(
                "Failed to read CID of block",
                "Invalid CID".to_string(),
            ));
        };

        if codec != 0x71 {
            return Err(value_error(
                "Failed to read CAR block",
                "Unsupported codec. For now we support only DAG-CBOR (0x71)".to_string(),
            ));
        }

        buf.advance(consumed);
        let cid_raw = &cid_bytes_before[..consumed];

        let block_result = to_pyobject(py, buf, 0, max_depth);
        let Ok(block) = block_result else {
            return Err(value_error(
                "Failed to read CAR block",
                block_result.unwrap_err().to_string(),
            ));
        };

        let key = PyBytes::new(py, cid_raw).into_pyobject(py)?;
        parsed_blocks.set_item(key, block)?;
    }

    Ok((header_obj, parsed_blocks))
}
