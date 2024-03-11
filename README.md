## Python IPLD

> This project aims to speed up [The AT Protocol SDK](https://github.com/MarshalX/atproto) by using Rust for the heavy lifting. Only atproto related parts are implemented first.

Code snippet:

```python
import libipld

# Decode a CID
print(libipld.decode_cid('bafyreig7jbijxpn4lfhvnvyuwf5u5jyhd7begxwyiqe7ingwxycjdqjjoa'))
# Output: {'hash': {'size': 32, 'code': 18, 'digest': b'\xdfHP\x9b\xbd\xbcYOV\xd7\x14\xb1{N\xa7\x07\x1f\xc2C^\xd8D\t\xf44\xd6\xbe\x04\x91\xc1)p'}, 'version': 1, 'codec': 113}

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

- Decode DAG-CBOR (`decode_dag_cbor(bytes) -> dict`, `decode_dag_cbor_multi(bytes) -> list[dict]`)
- Encode DAG-CBOR (`encode_dag_cbor(obj) -> bytes`)
- Decode CID (`decode_cid(str) -> dict`)
- Decode CAR (`decode_car(bytes) -> tuple[dict, dict[str, dict]]`). Returns a header and blocks mapped by CID.
- Decode Multibase (`decode_multibase(str) -> tuple[str, bytes]`). Returns base and data.
- Encode Multibase (`encode_multibase(str, bytes) -> str`). Accepts base and data.

Note: stub file will be provided in the future.

## Requirements

- Python 3.7 or higher.

## Installing

You can install or upgrade `libipld` via

```bash
pip install libipld
```

### Contributing

Contributions of all sizes are welcome.

### License

MIT
