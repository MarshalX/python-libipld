use pyo3::basic::CompareOp;
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::sync::PyOnceLock;
use pyo3::types::*;

use ::cid::multibase::Base;

use crate::cid::{cid_error, is_canonical_cid};
use crate::convert::with_bytes;
use crate::error::value_error;
use crate::ffi::string::from_bytes;

// version + codec (10) + multihash code (10) + size + 64-byte digest
pub(crate) const MAX_CID_LEN: usize = 86;
const MAX_DIGEST_LEN: usize = 64;

#[pyclass(module = "libipld", name = "CID", frozen)]
pub struct Cid {
    len: u8,
    buf: [u8; MAX_CID_LEN],
    string: PyOnceLock<Py<PyString>>,
}

#[pyclass(module = "libipld", name = "Multihash", frozen, eq, hash)]
#[derive(PartialEq, Hash)]
pub struct Multihash {
    code: u64,
    size: u8,
    buf: [u8; MAX_DIGEST_LEN],
}

#[pymethods]
impl Multihash {
    #[getter]
    fn code(&self) -> u64 {
        self.code
    }

    #[getter]
    fn size(&self) -> u8 {
        self.size
    }

    #[getter]
    fn digest<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.buf[..self.size as usize])
    }

    fn __repr__(&self) -> String {
        let digest: String = self.buf[..self.size as usize]
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        format!(
            "Multihash(code={}, size={}, digest={digest})",
            self.code, self.size
        )
    }

    fn __reduce__(&self) -> PyResult<()> {
        Err(PyTypeError::new_err("cannot pickle 'Multihash' object"))
    }

    fn __copy__(slf: Bound<'_, Self>) -> Bound<'_, Self> {
        slf
    }

    fn __deepcopy__<'py>(slf: Bound<'py, Self>, _memo: &Bound<'py, PyAny>) -> Bound<'py, Self> {
        slf
    }
}

fn invalid_cid(detail: &str) -> PyErr {
    value_error("Failed to decode CID", detail.to_string())
}

impl Cid {
    /// `bytes` must pass `is_canonical_cid`.
    // Never inlined: the buffer would grow every recursive decoder frame.
    #[inline(never)]
    pub(crate) fn from_valid<'py>(py: Python<'py>, bytes: &[u8]) -> PyResult<Bound<'py, Cid>> {
        Bound::new(py, Cid::new_unchecked(bytes, PyOnceLock::new()))
    }

    fn new_unchecked(bytes: &[u8], string: PyOnceLock<Py<PyString>>) -> Self {
        let mut buf = [0u8; MAX_CID_LEN];
        buf[..bytes.len()].copy_from_slice(bytes);
        Cid {
            len: bytes.len() as u8,
            buf,
            string,
        }
    }

    /// Empty only for an instance made by `object.__new__` on PyPy.
    #[inline]
    pub(crate) fn as_bytes(&self) -> &[u8] {
        &self.buf[..self.len as usize]
    }

    fn valid_bytes(&self) -> PyResult<&[u8]> {
        match self.as_bytes() {
            [] => Err(PyValueError::new_err("Uninitialized CID")),
            b => Ok(b),
        }
    }

    fn parse(py: Python<'_>, data: &Bound<'_, PyAny>) -> PyResult<Self> {
        if let Ok(s) = data.cast::<PyString>() {
            let text = s.to_str()?;
            let cid = decode_str(text).ok_or_else(|| invalid_cid("Invalid CID"))?;
            if !is_canonical_cid(&cid) {
                return Err(invalid_cid(cid_error(&cid)));
            }
            let string = PyOnceLock::new();
            if s.is_exact_instance_of::<PyString>() && text.as_bytes() == string_form(&cid) {
                let _ = string.set(py, s.clone().unbind());
            }
            return Ok(Cid::new_unchecked(&cid, string));
        }
        if !(data.is_instance_of::<PyBytes>() || data.is_instance_of::<PyByteArray>()) {
            return Err(PyTypeError::new_err(format!(
                "CID() argument must be str, bytes, bytearray or CID, not {}",
                data.get_type().name()?
            )));
        }
        with_bytes(data, |b| {
            if !is_canonical_cid(b) {
                return Err(invalid_cid(cid_error(b)));
            }
            Ok(Cid::new_unchecked(b, PyOnceLock::new()))
        })
    }

    pub(crate) fn string<'py>(&self, py: Python<'py>) -> PyResult<&Bound<'py, PyString>> {
        let bytes = self.valid_bytes()?;
        Ok(self
            .string
            .get_or_try_init(py, || {
                from_bytes(py, &string_form(bytes)).map(Bound::unbind)
            })?
            .bind(py))
    }

    // version, codec, multihash code, digest size
    fn varints(&self) -> PyResult<[u64; 4]> {
        let b = self.valid_bytes()?;
        if b[0] == 0x12 {
            return Ok([0, 0x70, 0x12, 32]);
        }
        let mut out = [0u64; 4];
        let mut pos = 0;
        for v in out.iter_mut() {
            let mut shift = 0;
            loop {
                let byte = b[pos];
                pos += 1;
                *v |= ((byte & 0x7f) as u64) << shift;
                shift += 7;
                if byte & 0x80 == 0 {
                    break;
                }
            }
        }
        Ok(out)
    }
}

fn decode_str(text: &str) -> Option<Vec<u8>> {
    if text.len() == 46 && text.starts_with("Qm") {
        return Base::Base58Btc.decode(text).ok();
    }
    let (_, bytes) = ::cid::multibase::decode(text).ok()?;
    (bytes.first() == Some(&1)).then_some(bytes)
}

const BASE32: &[u8; 32] = b"abcdefghijklmnopqrstuvwxyz234567";

fn string_form(cid: &[u8]) -> Vec<u8> {
    if cid[0] == 0x12 {
        return Base::Base58Btc.encode(cid).into_bytes();
    }
    let mut out = Vec::with_capacity(1 + (cid.len() * 8).div_ceil(5));
    out.push(b'b');
    let (mut acc, mut bits) = (0u64, 0u32);
    for &byte in cid {
        acc = (acc << 8) | byte as u64;
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            out.push(BASE32[((acc >> bits) & 31) as usize]);
        }
    }
    if bits > 0 {
        out.push(BASE32[((acc << (5 - bits)) & 31) as usize]);
    }
    out
}

#[pymethods]
impl Cid {
    #[new]
    fn new(py: Python<'_>, data: &Bound<'_, PyAny>) -> PyResult<Py<Self>> {
        if let Ok(cid) = data.cast::<Cid>() {
            return Ok(cid.clone().unbind());
        }
        Py::new(py, Self::parse(py, data)?)
    }

    #[classmethod]
    fn decode(
        _cls: &Bound<'_, PyType>,
        py: Python<'_>,
        data: &Bound<'_, PyAny>,
    ) -> PyResult<Py<Self>> {
        Self::new(py, data)
    }

    #[getter]
    fn version(&self) -> PyResult<u64> {
        Ok(self.varints()?[0])
    }

    #[getter]
    fn codec(&self) -> PyResult<u64> {
        Ok(self.varints()?[1])
    }

    #[getter]
    fn hash(&self) -> PyResult<Multihash> {
        let [_, _, code, size] = self.varints()?;
        let b = self.as_bytes();
        let mut buf = [0u8; MAX_DIGEST_LEN];
        buf[..size as usize].copy_from_slice(&b[b.len() - size as usize..]);
        Ok(Multihash {
            code,
            size: size as u8,
            buf,
        })
    }

    fn encode<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyString>> {
        self.string(py).cloned()
    }

    fn __str__<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyString>> {
        self.string(py).cloned()
    }

    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        Ok(format!("CID('{}')", self.string(py)?.to_str()?))
    }

    fn __bytes__<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        Ok(PyBytes::new(py, self.valid_bytes()?))
    }

    fn __hash__(&self) -> isize {
        let b = self.as_bytes();
        unsafe {
            pyo3::ffi::compat::Py_HashBuffer(b.as_ptr().cast(), b.len() as pyo3::ffi::Py_ssize_t)
        }
    }

    fn __richcmp__(&self, other: &Bound<'_, PyAny>, op: CompareOp) -> Py<PyAny> {
        let py = other.py();
        let Ok(other) = other.cast::<Cid>() else {
            return py.NotImplemented();
        };
        let equal = self.as_bytes() == other.get().as_bytes();
        match op {
            CompareOp::Eq => PyBool::new(py, equal).to_owned().into_any().unbind(),
            CompareOp::Ne => PyBool::new(py, !equal).to_owned().into_any().unbind(),
            _ => py.NotImplemented(),
        }
    }

    fn __reduce__<'py>(
        slf: &Bound<'py, Self>,
    ) -> PyResult<(Bound<'py, PyType>, (Bound<'py, PyBytes>,))> {
        Ok((slf.get_type(), (slf.get().__bytes__(slf.py())?,)))
    }

    fn __copy__(slf: Bound<'_, Self>) -> Bound<'_, Self> {
        slf
    }

    fn __deepcopy__<'py>(slf: Bound<'py, Self>, _memo: &Bound<'py, PyAny>) -> Bound<'py, Self> {
        slf
    }
}
