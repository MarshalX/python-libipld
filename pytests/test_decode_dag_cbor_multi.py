import os

import libipld
import pytest

from conftest import load_json_data_fixtures


_ROUNDTRIP_DATA_DIR = os.path.join(os.path.dirname(__file__), '..', 'data', 'roundtrip')


def combined_data():
    data = load_json_data_fixtures(_ROUNDTRIP_DATA_DIR)

    result = []

    i1 = 0
    i2 = 1
    while i2 < len(data):
        result.append((f'{data[i1][0]} + {data[i2][0]}', [data[i1][1], data[i2][1]]))
        i1 += 1
        i2 += 1

    return result


@pytest.mark.parametrize('data', combined_data(), ids=lambda data: data[0])
def test_decode_dag_cbor_multi(data) -> None:
    _, objects = data

    encoded = b''
    for obj in objects:
        # encode multiple objects into a single byte stream
        encoded += libipld.encode_dag_cbor(obj)

    decoded = libipld.decode_dag_cbor_multi(encoded)
    assert len(decoded) == len(objects)
    assert decoded == objects


def test_decode_dag_cbor_multi_corrupt_trailing_data_error() -> None:
    encoded = libipld.encode_dag_cbor({'abc': 1})

    with pytest.raises(ValueError) as exc_info:
        # 9b0000000040000000 - array claiming 2**30 elements with no payload
        libipld.decode_dag_cbor_multi(encoded + bytes.fromhex('9b0000000040000000'))

    assert 'Failed to decode DAG-CBOR' in str(exc_info.value)


def test_decode_dag_cbor_multi_truncated_object_error() -> None:
    encoded = libipld.encode_dag_cbor({'abc': 1})

    with pytest.raises(ValueError):
        # second object is truncated mid-string
        libipld.decode_dag_cbor_multi(encoded + bytes.fromhex('6361'))
