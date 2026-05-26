## Python IPLD

> This project aims to speed up [The AT Protocol SDK](https://github.com/MarshalX/atproto) by using Rust for the heavy lifting. Only atproto related parts are implemented first.

Code snippet:

```python
import libipld

# CID
print(libipld.decode_cid('bafyreig7jbijxpn4lfhvnvyuwf5u5jyhd7begxwyiqe7ingwxycjdqjjoa'))
# Output: {'hash': {'size': 32, 'code': 18, 'digest': b'\xdfHP\x9b\xbd\xbcYOV\xd7\x14\xb1{N\xa7\x07\x1f\xc2C^\xd8D\t\xf44\xd6\xbe\x04\x91\xc1)p'}, 'version': 1, 'codec': 113}
print(libipld.encode_cid(b'\x01q\x12 \xb6\x81\x1a\x1d\x7f\x8c\x17\x91\xdam\x1bO\x13m\xc0\xe2&y\xea\xfe\xaaX\xd6M~/\xaa\xd5\x89\x0e\x9d\x9c'))
# Output: bafyreifwqenb274mc6i5u3i3j4jw3qhcez46v7vkldle27rpvlkysdu5tq

# DAG-CBOR
print(libipld.decode_dag_cbor(b'\xa2aa\x0cabfhello!'))
# Output: {'a': 12, 'b': 'hello!'}
print(libipld.encode_dag_cbor({'a': 12, 'b': 'hello!'}))
# Output: b'\xa2aa\x0cabfhello!'

# multibase
print(libipld.decode_multibase('ueWVzIG1hbmkgIQ'))
# Output: ('u', b'yes mani !')
print(libipld.encode_multibase('u', b'yes mani !'))
# Output: ueWVzIG1hbmkgIQ
```

### Features

#### 🔗 CID (Content Identifier) Operations
- **`decode_cid(data: str | bytes) -> dict`** - Decode CIDs from string representation (e.g., `'bafy...'`) or raw bytes into structured data containing version, codec, and hash information
- **`encode_cid(data: str | bytes) -> str`** - Encode CID raw bytes to string representation, or return string CIDs as-is

#### 📦 DAG-CBOR (Directed Acyclic Graph CBOR) Operations  
- **`decode_dag_cbor(data: bytes) -> Any`** - Decode DAG-CBOR binary data into Python objects (dicts, lists, primitives)
- **`decode_dag_cbor_multi(data: bytes) -> list[Any]`** - Decode multiple concatenated DAG-CBOR objects from a single byte stream
- **`encode_dag_cbor(data: Any) -> bytes`** - Encode Python objects into DAG-CBOR binary format

#### 🌐 Multibase Operations
- **`decode_multibase(data: str) -> tuple[str, bytes]`** - Decode multibase-encoded strings, returning the base identifier and decoded data
- **`encode_multibase(code: str, data: str | bytes) -> str`** - Encode data using specified multibase encoding (e.g., base58btc with code `'u'`)

#### 🚗 CAR (Content Addressable Archives) Operations
- **`decode_car(data: bytes) -> tuple[dict, dict[bytes, dict]]`** - Decode CAR files into header metadata and a mapping of CID bytes to block data

### Requirements

- Python 3.8 or higher.

### Installing

You can install or upgrade `libipld` via

```bash
pip install -U libipld
```

### Performance

Benchmarks against [`cbrrr`](https://github.com/DavidBuchanan314/dag-cbrrr) (C), [`py-ipld-dag`](https://github.com/ipld/py-ipld-dag) (Python wrapper over Rust-backed [`cbor2`](https://github.com/agronholm/cbor2)) and [`dag_cbor`](https://github.com/hashberg-io/dag-cbor) (pure Python), measured on the four classic [nativejson-benchmark](https://github.com/miloyip/nativejson-benchmark) fixtures (round-tripped through DAG-CBOR). Bars are operations/second relative to pure-Python `dag_cbor`; higher is better.

Measured on Apple M1, macOS 15 (Darwin 24.6.0), CPython 3.14.0, `libipld` installed from PyPI (PGO + LTO wheel).

#### Deserialization

![deserialization](https://raw.githubusercontent.com/MarshalX/python-libipld/main/benchmark/deserialization.png)

#### Serialization

![serialization](https://raw.githubusercontent.com/MarshalX/python-libipld/main/benchmark/serialization.png)

Reproduce locally:

```bash
cd benchmark && ./run.sh
```

See [`benchmark/README.md`](./benchmark/README.md) for details.

### Contributing

Contributions of all sizes are welcome. See [`CONTRIBUTING.md`](./CONTRIBUTING.md) for setup, build, test, and benchmarking workflow.

### License

MIT – see [LICENSE](./LICENSE).
