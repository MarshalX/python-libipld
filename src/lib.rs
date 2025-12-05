use std::io::{self, Cursor, Write};

use anyhow::{anyhow, Result};
use cbor4ii::core::{
    dec::{self, Decode, Read},
    enc::{self, Encode},
    major, marker, types,
};
use cid::{multibase, Cid};
use pyo3::pybacked::PyBackedStr;
use pyo3::{ffi, prelude::*, types::*, BoundObject, Python};

struct BufWriter<W: io::Write>(io::BufWriter<W>);

impl<W: Write> BufWriter<W> {
    pub fn new(inner: W) -> Self {
        BufWriter(io::BufWriter::new(inner))
    }

    pub fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }

    pub fn get_ref(&self) -> &W {
        self.0.get_ref()
    }
}

impl<W: Write> enc::Write for BufWriter<W> {
    type Error = io::Error;

    #[inline]
    fn push(&mut self, input: &[u8]) -> Result<(), Self::Error> {
        self.0.write_all(input)?;
        Ok(())
    }
}

// Based on cbor4ii/src/utils.rs.
/// An in-memory reader.
struct SliceReader<'a> {
    buf: &'a [u8],
}

impl SliceReader<'_> {
    fn new(buf: &[u8]) -> SliceReader<'_> {
        SliceReader { buf }
    }
}

impl<'de> dec::Read<'de> for SliceReader<'de> {
    type Error = core::convert::Infallible;

    #[inline]
    fn fill<'b>(&'b mut self, want: usize) -> Result<dec::Reference<'de, 'b>, Self::Error> {
        let len = core::cmp::min(self.buf.len(), want);
        Ok(dec::Reference::Long(&self.buf[..len]))
    }

    #[inline]
    fn advance(&mut self, n: usize) {
        let len = core::cmp::min(self.buf.len(), n);
        self.buf = &self.buf[len..];
    }
}

struct PrefixedCidBytes<'a>(&'a [u8]);

impl<'a> Encode for PrefixedCidBytes<'a> {
    fn encode<W: enc::Write>(&self, w: &mut W) -> Result<(), enc::Error<W::Error>> {
        // length prefix for bytes: 1 (leading 0) + payload
        types::Bytes::bounded(1 + self.0.len(), w)?;
        w.push(&[0x00])?;
        w.push(self.0)?;
        Ok(())
    }
}

fn cid_hash_to_pydict<'py>(py: Python<'py>, cid: &Cid) -> PyResult<Bound<'py, PyDict>> {
    let hash = cid.hash();
    let dict_obj = PyDict::new(py);

    dict_obj.set_item("code", hash.code())?;
    dict_obj.set_item("size", hash.size())?;
    dict_obj.set_item("digest", PyBytes::new(py, hash.digest()))?;

    Ok(dict_obj)
}

fn cid_to_pydict<'py>(py: Python<'py>, cid: &Cid) -> PyResult<Bound<'py, PyDict>> {
    let dict_obj = PyDict::new(py);

    dict_obj.set_item("version", cid.version() as u64)?;
    dict_obj.set_item("codec", cid.codec())?;
    dict_obj.set_item("hash", cid_hash_to_pydict(py, cid)?)?;
    Ok(dict_obj)
}

fn map_key_cmp(a: &[u8], b: &[u8]) -> std::cmp::Ordering {
    /* The keys in every map must be sorted length-first by the byte representation of the string keys, where:
    - If two keys have different lengths, the shorter one sorts earlier;
    - If two keys have the same length, the one with the lower value in (byte-wise) lexical order sorts earlier.
     */
    if a.len() != b.len() {
        a.len().cmp(&b.len())
    } else {
        a.cmp(b)
    }
}

fn sort_map_keys(keys: &Bound<PyList>, len: usize) -> Result<Vec<(PyBackedStr, usize)>> {
    // Returns key and index.
    let mut keys_str = Vec::with_capacity(len);
    for i in 0..len {
        let item = unsafe { keys.get_item_unchecked(i) };
        let key = match item.cast::<PyString>() {
            Ok(k) => k.to_owned(),
            Err(_) => return Err(anyhow!("Map keys must be strings")),
        };
        let backed_str = match PyBackedStr::try_from(key) {
            Ok(bs) => bs,
            Err(_) => return Err(anyhow!("Failed to convert PyString to PyBackedStr")),
        };
        keys_str.push((backed_str, i));
    }

    if keys_str.len() < 2 {
        return Ok(keys_str);
    }

    keys_str.sort_by(|a, b| {
        // sort_unstable_by performs bad
        let (s1, _) = a;
        let (s2, _) = b;

        // sorted length-first by the byte representation of the string keys
        if s1.len() != s2.len() {
            s1.len().cmp(&s2.len())
        } else {
            s1.cmp(s2)
        }
    });

    Ok(keys_str)
}

fn get_bytes_from_py_any<'py>(obj: &'py Bound<'py, PyAny>) -> PyResult<&'py [u8]> {
    if let Ok(b) = obj.cast::<PyBytes>() {
        Ok(b.as_bytes())
    } else if let Ok(ba) = obj.cast::<PyByteArray>() {
        Ok(unsafe { ba.as_bytes() })
    } else if let Ok(s) = obj.cast::<PyString>() {
        Ok(s.to_str()?.as_bytes())
    } else {
        Err(get_err(
            "Failed to encode multibase",
            "Unsupported data type".to_string(),
        ))
    }
}

// Based on cbor4ii code.
fn peek_one<'de, R: dec::Read<'de>>(r: &mut R) -> Result<u8>
where
    R::Error: Send + Sync,
{
    r.fill(1)?
        .as_ref()
        .first()
        .copied()
        .ok_or_else(|| anyhow!("end of data"))
}

fn decode_dag_cbor_to_pyobject<'de, R: dec::Read<'de>>(
    py: Python,
    r: &mut R,
    depth: usize,
) -> Result<Py<PyAny>>
where
    R::Error: Send + Sync,
{
    unsafe {
        if depth > ffi::Py_GetRecursionLimit() as usize {
            PyErr::new::<pyo3::exceptions::PyRecursionError, _>(
                "RecursionError: maximum recursion depth exceeded in DAG-CBOR decoding",
            )
            .restore(py);

            return Err(anyhow!("Maximum recursion depth exceeded"));
        }
    }

    let byte = peek_one(r)?;
    Ok(match dec::if_major(byte) {
        major::UNSIGNED => u64::decode(r)?.into_pyobject(py)?.into(),
        major::NEGATIVE => i128::decode(r)?.into_pyobject(py)?.into(),
        major::BYTES => PyBytes::new(py, <types::Bytes<&[u8]>>::decode(r)?.0)
            .into_pyobject(py)?
            .into(),
        major::STRING => {
            // The UTF-8 validation is done when it's converted into a Python string
            PyString::from_bytes(
                py,
                <types::UncheckedStr<&[u8]>>::decode(r)
                    .map_err(|_| anyhow!("Cannot decode as bytes"))?
                    .0,
            )?
            .into()
        }
        major::ARRAY => {
            let len: ffi::Py_ssize_t = types::Array::len(r)?
                .ok_or_else(|| anyhow!("Array must contain length"))?
                .try_into()?;

            unsafe {
                let ptr = ffi::PyList_New(len);

                for i in 0..len {
                    ffi::PyList_SET_ITEM(
                        ptr,
                        i,
                        decode_dag_cbor_to_pyobject(py, r, depth + 1)?.into_ptr(),
                    );
                }

                let list: Bound<'_, PyList> = Bound::from_owned_ptr(py, ptr).cast_into_unchecked();
                list.into_pyobject(py)?.into()
            }
        }
        major::MAP => {
            let len = types::Map::len(r)?.ok_or_else(|| anyhow!("Map must contain length"))?;
            let dict = PyDict::new(py);

            let mut prev_key: Option<&[u8]> = None;
            for _ in 0..len {
                // DAG-CBOR keys are always strings. Python does the UTF-8 validation when creating
                // the string.
                let key = <types::UncheckedStr<&[u8]>>::decode(r)
                    .map_err(|_| anyhow!("Map keys must be strings"))?
                    .0;

                if let Some(prev_key) = prev_key {
                    // it cares about duplicated keys too thanks to Ordering::Equal
                    if map_key_cmp(prev_key, key) != std::cmp::Ordering::Less {
                        return Err(anyhow!("Map keys must be sorted and unique"));
                    }
                }

                let key_py = PyString::from_bytes(py, key)?;
                prev_key = Some(key);

                let value_py = decode_dag_cbor_to_pyobject(py, r, depth + 1)?;
                dict.set_item(key_py, value_py)?;
            }

            dict.into_pyobject(py)?.into()
        }
        major::TAG => {
            let value = types::Tag::tag(r)?;
            if value != 42 {
                return Err(anyhow!("Non-42 tags are not supported"));
            }

            let cid = <types::Bytes<&[u8]>>::decode(r)?.0;

            // we expect CIDs to have a leading zero byte
            if cid.len() <= 1 || cid[0] != 0 {
                return Err(anyhow!("Invalid CID"));
            } 
        
            let cid_without_prefix = &cid[1..];
            if Cid::try_from(cid_without_prefix).is_err() {
                return Err(anyhow!("Invalid CID"));
            }

            PyBytes::new(py, cid_without_prefix).into_pyobject(py)?.into()
        }
        major::SIMPLE => match byte {
            // FIXME(MarshalX): should be more clear for bool?
            marker::FALSE => {
                r.advance(1);
                false.into_pyobject(py)?.into_any().unbind()
            }
            marker::TRUE => {
                r.advance(1);
                true.into_pyobject(py)?.into_any().unbind()
            }
            marker::NULL => {
                r.advance(1);
                py.None()
            }
            marker::F32 => {
                let value = f32::decode(r)?;
                if !value.is_finite() {
                    return Err(anyhow!(
                        "Number out of range for f32 (NaNs are forbidden)".to_string()
                    ));
                }
                value.into_pyobject(py)?.into()
            }
            marker::F64 => {
                let value = f64::decode(r)?;
                if !value.is_finite() {
                    return Err(anyhow!(
                        "Number out of range for f64 (NaNs are forbidden)".to_string()
                    ));
                }
                value.into_pyobject(py)?.into()
            }
            _ => return Err(anyhow!("Unsupported major type".to_string())),
        },
        _ => return Err(anyhow!("Invalid major type".to_string())),
    })
}

fn encode_dag_cbor_from_pyobject<'py, W: enc::Write>(
    _py: Python<'py>,
    obj: &Bound<'py, PyAny>,
    w: &mut W,
) -> Result<()>
where
    W::Error: Send + Sync,
{
    /* Order is important for performance!

    Fast checks go first:
    - None
    - bool
    - int
    - list
    - dict
    - str
    Then slow checks:
    - bytes
    - float
     */

    if obj.is_none() {
        types::Null.encode(w)?;

        Ok(())
    } else if obj.is_instance_of::<PyBool>() {
        obj.is_truthy()?.encode(w)?;

        Ok(())
    } else if obj.is_instance_of::<PyInt>() {
        let i: i128 = obj.extract()?;

        if i.is_negative() {
            if -(i + 1) > u64::MAX as i128 {
                return Err(anyhow!("Number out of range"));
            }

            types::Negative(-(i + 1) as u64).encode(w)?;
        } else {
            if i > u64::MAX as i128 {
                return Err(anyhow!("Number out of range"));
            }

            (i as u64).encode(w)?;
        }

        Ok(())
    } else if let Ok(l) = obj.cast::<PyList>() {
        let len = l.len();

        types::Array::bounded(len, w)?;

        for i in 0..len {
            let item = unsafe { l.get_item_unchecked(i) };
            encode_dag_cbor_from_pyobject(_py, &item, w)?;
        }

        Ok(())
    } else if let Ok(map) = obj.cast::<PyDict>() {
        let len = map.len();
        let keys = sort_map_keys(&map.keys(), len)?;
        let values = map.values();

        types::Map::bounded(len, w)?;

        for (key, i) in keys {
            key.get(..)
                .ok_or_else(|| anyhow!("Invalid string range while encoding map key"))?
                .encode(w)?;
            let value = unsafe { values.get_item_unchecked(i) };
            encode_dag_cbor_from_pyobject(_py, &value, w)?;
        }

        Ok(())
    } else if let Ok(f) = obj.cast::<PyFloat>() {
        let v = f.value();
        if !v.is_finite() {
            return Err(anyhow!("Number out of range"));
        }

        v.encode(w)?;

        Ok(())
    } else if let Ok(b) = obj.cast::<PyBytes>() {
        // FIXME (MarshalX): it's not efficient to try to parse it as CID
        let bytes = b.as_bytes();
        let cid = Cid::try_from(bytes);
        if cid.is_ok() {
            // by providing custom encoding we avoid extra allocation
            types::Tag(42, PrefixedCidBytes(bytes)).encode(w)?;
        } else {
            types::Bytes(bytes).encode(w)?;
        }

        Ok(())
    } else if let Ok(s) = obj.cast::<PyString>() {
        s.to_str()?.encode(w)?;

        Ok(())
    } else {
        Err(anyhow!("Unknown tag"))
    }
}

#[pyfunction]
fn decode_dag_cbor_multi<'py>(py: Python<'py>, data: &[u8]) -> PyResult<Bound<'py, PyList>> {
    let mut reader = SliceReader::new(data);
    let decoded_parts = PyList::empty(py);

    loop {
        let py_object = decode_dag_cbor_to_pyobject(py, &mut reader, 0);
        if let Ok(py_object) = py_object {
            decoded_parts.append(py_object)?;
        } else {
            break;
        }
    }

    Ok(decoded_parts)
}

#[inline]
fn read_u64_leb128<'de, R: dec::Read<'de>>(r: &mut R) -> Result<u64>
where
    R::Error: Send + Sync,
{
    let mut result: u64 = 0;
    let mut shift = 0;

    loop {
        let byte =
            peek_one(r).map_err(|_| anyhow!("Unexpected EOF while reading ULEB128 number."))?;
        r.advance(1);

        if shift == 63 && byte != 0x00 && byte != 0x01 {
            // consume remaining continuation bytes so reader stays in sync
            let mut b = byte;
            while b & 0x80 != 0 {
                b = peek_one(r).map_err(|_| {
                    anyhow!("Unexpected EOF while skipping overflowing ULEB128 number.")
                })?;
                r.advance(1);
            }
            return Err(anyhow!("ULEB128 overflow"));
        }

        let low_bits = (byte & !0x80) as u64;
        result |= low_bits << shift;

        if byte & 0x80 == 0 {
            return Ok(result);
        }

        shift += 7;
    }
}

#[pyfunction]
pub fn decode_car<'py>(py: Python<'py>, data: &[u8]) -> PyResult<(Py<PyAny>, Bound<'py, PyDict>)> {
    let buf = &mut SliceReader::new(data);

    if read_u64_leb128(buf).is_err() {
        return Err(get_err(
            "Failed to read CAR header",
            "Invalid uvarint".to_string(),
        ));
    }
    let Ok(header_obj) = decode_dag_cbor_to_pyobject(py, buf, 0) else {
        return Err(get_err(
            "Failed to read CAR header",
            "Invalid DAG-CBOR".to_string(),
        ));
    };

    let header = header_obj.cast_bound::<PyDict>(py)?;

    let Some(version) = header.get_item("version")? else {
        return Err(get_err(
            "Failed to read CAR header",
            "Version is None".to_string(),
        ));
    };
    if version.cast::<PyInt>()?.extract::<u64>()? != 1 {
        return Err(get_err(
            "Failed to read CAR header",
            "Unsupported version. Version must be 1".to_string(),
        ));
    }

    let Some(roots) = header.get_item("roots")? else {
        return Err(get_err(
            "Failed to read CAR header",
            "Roots is None".to_string(),
        ));
    };
    if roots.cast::<PyList>()?.len() == 0 {
        return Err(get_err(
            "Failed to read CAR header",
            "Roots is empty. Must be at least one".to_string(),
        ));
    }

    // FIXME (MarshalX): we are not verifying if the roots are valid CIDs

    let parsed_blocks = PyDict::new(py);

    loop {
        if read_u64_leb128(buf).is_err() {
            // FIXME (MarshalX): we are not raising an error here because of possible EOF
            break;
        }

        let cid_bytes_before = buf.buf;
        let mut cursor = Cursor::new(cid_bytes_before);

        let cid_result = Cid::read_bytes(&mut cursor);
        let Ok(cid) = cid_result else {
            return Err(get_err(
                "Failed to read CID of block",
                cid_result.unwrap_err().to_string(),
            ));
        };

        if cid.codec() != 0x71 {
            return Err(get_err(
                "Failed to read CAR block",
                "Unsupported codec. For now we support only DAG-CBOR (0x71)".to_string(),
            ));
        }

        let consumed = cursor.position() as usize;
        buf.advance(consumed);
        let cid_raw = &cid_bytes_before[..consumed];

        let block_result = decode_dag_cbor_to_pyobject(py, buf, 0);
        let Ok(block) = block_result else {
            return Err(get_err(
                "Failed to read CAR block",
                block_result.unwrap_err().to_string(),
            ));
        };

        let key = PyBytes::new(py, cid_raw).into_pyobject(py)?;
        parsed_blocks.set_item(key, block)?;
    }

    Ok((header_obj, parsed_blocks))
}

#[pyfunction]
pub fn decode_dag_cbor(py: Python, data: &[u8]) -> PyResult<Py<PyAny>> {
    let mut reader = SliceReader::new(data);
    let py_object = decode_dag_cbor_to_pyobject(py, &mut reader, 0);
    if let Ok(py_object) = py_object {
        // check for any remaining data in the reader
        if reader.fill(1)?.as_ref().is_empty() {
            Ok(py_object)
        } else {
            Err(get_err(
                "Failed to decode DAG-CBOR",
                "Invalid DAG-CBOR: contains multiple objects (CBOR sequence)".to_string(),
            ))
        }
    } else {
        let err = get_err(
            "Failed to decode DAG-CBOR",
            py_object.unwrap_err().to_string(),
        );

        if let Some(py_err) = PyErr::take(py) {
            py_err.set_cause(py, Option::from(err));
            // in case something set global interpreter’s error,
            // for example C FFI function, we should return it
            // the real case: RecursionError (set by Py_EnterRecursiveCall)
            Err(py_err)
        } else {
            Err(err)
        }
    }
}

#[pyfunction]
pub fn encode_dag_cbor<'py>(
    py: Python<'py>,
    data: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyBytes>> {
    let mut buf = &mut BufWriter::new(Vec::new());
    if let Err(e) = encode_dag_cbor_from_pyobject(py, data, &mut buf) {
        return Err(get_err("Failed to encode DAG-CBOR", e.to_string()));
    }
    if let Err(e) = buf.flush() {
        return Err(get_err("Failed to flush buffer", e.to_string()));
    }
    Ok(PyBytes::new(py, buf.get_ref()))
}

fn get_cid_from_py_any(data: &Bound<PyAny>) -> PyResult<Cid> {
    let cid = if let Ok(s) = data.cast::<PyString>() {
        Cid::try_from(s.to_str()?)
    } else {
        Cid::try_from(get_bytes_from_py_any(data)?)
    };

    if let Ok(cid) = cid {
        Ok(cid)
    } else {
        Err(get_err(
            "Failed to decode CID",
            cid.unwrap_err().to_string(),
        ))
    }
}

#[pyfunction]
fn decode_cid<'py>(py: Python<'py>, data: &Bound<PyAny>) -> PyResult<Bound<'py, PyDict>> {
    cid_to_pydict(py, &get_cid_from_py_any(data)?)
}

#[pyfunction]
fn encode_cid<'py>(py: Python<'py>, data: &Bound<PyAny>) -> PyResult<Bound<'py, PyString>> {
    Ok(PyString::new(
        py,
        get_cid_from_py_any(data)?.to_string().as_str(),
    ))
}

#[pyfunction]
fn decode_multibase<'py>(py: Python<'py>, data: &str) -> PyResult<(char, Bound<'py, PyBytes>)> {
    let base = multibase::decode(data);
    if let Ok((base, data)) = base {
        Ok((base.code(), PyBytes::new(py, &data)))
    } else {
        Err(get_err(
            "Failed to decode multibase",
            base.unwrap_err().to_string(),
        ))
    }
}

#[pyfunction]
fn encode_multibase(code: char, data: &Bound<PyAny>) -> PyResult<String> {
    let data_bytes = get_bytes_from_py_any(data)?;
    let base = multibase::Base::from_code(code);
    if let Ok(base) = base {
        Ok(multibase::encode(base, data_bytes))
    } else {
        Err(get_err(
            "Failed to encode multibase",
            base.unwrap_err().to_string(),
        ))
    }
}

fn get_err(msg: &str, err: String) -> PyErr {
    PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}. {}", msg, err))
}

#[pymodule]
#[pyo3(name = "_libipld")]
fn libipld(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(decode_cid, m)?)?;
    m.add_function(wrap_pyfunction!(encode_cid, m)?)?;

    m.add_function(wrap_pyfunction!(decode_car, m)?)?;

    m.add_function(wrap_pyfunction!(decode_dag_cbor, m)?)?;
    m.add_function(wrap_pyfunction!(decode_dag_cbor_multi, m)?)?;
    m.add_function(wrap_pyfunction!(encode_dag_cbor, m)?)?;

    m.add_function(wrap_pyfunction!(decode_multibase, m)?)?;
    m.add_function(wrap_pyfunction!(encode_multibase, m)?)?;

    Ok(())
}
