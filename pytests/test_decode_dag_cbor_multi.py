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
