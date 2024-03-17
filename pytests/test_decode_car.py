import os

import libipld
import pytest

from conftest import load_car_fixture

_DID = os.environ.get('CAR_REPO_DID', 'did:plc:w4es6sfh43zlht3bgrzi5qzq')  # default is public bot in bsky.app
_REPO_CAR_PATH = os.path.join(os.path.dirname(__file__), '..', 'data', 'repo.car')


@pytest.fixture(scope='session')
def car() -> bytes:
    return load_car_fixture(_DID, _REPO_CAR_PATH)


def test_decode_car(benchmark, car) -> None:
    header, blocks = benchmark(libipld.decode_car, car)

    assert 1 == header['version']
    assert isinstance(header['roots'], list)
    assert 1 == len(header['roots'])

    assert isinstance(blocks, dict)
    assert all(isinstance(k, bytes) for k in blocks.keys())
    assert all(len(k) == 36 for k in blocks.keys())
    assert all(isinstance(v, dict) for v in blocks.values())
    assert all(v for v in blocks.values())  # not empty dict


def test_decode_car_invalid_header_len() -> None:
    with pytest.raises(ValueError) as exc_info:
        libipld.decode_car(b'')

    assert 'Invalid uvarint' in str(exc_info.value)


def test_decode_car_invalid_header_type() -> None:
    with pytest.raises(TypeError) as exc_info:
        header_len = bytes.fromhex('33')  # 3
        header_obj = libipld.encode_dag_cbor('strInsteadOfObj')
        libipld.decode_car(header_len + header_obj)

    assert "cannot be converted to 'PyDict'" in str(exc_info.value)


def test_decode_car_invalid_header_version_key() -> None:
    with pytest.raises(ValueError) as exc_info:
        header_len = bytes.fromhex('33')  # 3
        header_obj = libipld.encode_dag_cbor({'blabla': 'blabla'})
        libipld.decode_car(header_len + header_obj)

    assert 'Version is None' in str(exc_info.value)


def test_decode_car_invalid_header_version_value() -> None:
    with pytest.raises(ValueError) as exc_info:
        header_len = bytes.fromhex('33')  # 3
        header_obj = libipld.encode_dag_cbor({'version': 2})
        libipld.decode_car(header_len + header_obj)

    assert 'Version must be 1' in str(exc_info.value)


def test_decode_car_invalid_header_roots_key() -> None:
    with pytest.raises(ValueError) as exc_info:
        header_len = bytes.fromhex('33')  # 3
        header_obj = libipld.encode_dag_cbor({'version': 1})
        libipld.decode_car(header_len + header_obj)

    assert 'Roots is None' in str(exc_info.value)


def test_decode_car_invalid_header_roots_value_type() -> None:
    with pytest.raises(TypeError) as exc_info:
        header_len = bytes.fromhex('33')  # 3
        header_obj = libipld.encode_dag_cbor({'version': 1, 'roots': 123})
        libipld.decode_car(header_len + header_obj)

    assert "cannot be converted to 'PyList'" in str(exc_info.value)


def test_decode_car_invalid_header_roots_value_empty_list() -> None:
    with pytest.raises(ValueError) as exc_info:
        header_len = bytes.fromhex('33')  # 3
        header_obj = libipld.encode_dag_cbor({'version': 1, 'roots': []})
        libipld.decode_car(header_len + header_obj)

    assert 'Roots is empty' in str(exc_info.value)


def test_decode_car_invalid_block_cid() -> None:
    with pytest.raises(ValueError) as exc_info:
        header_len = bytes.fromhex('33')  # 3
        header_obj = libipld.encode_dag_cbor({'version': 1, 'roots': ['blabla']})
        block1 = bytes.fromhex('33') + b'invalidSid'

        libipld.decode_car(header_len + header_obj + block1)

    assert 'Failed to read CID of block' in str(exc_info.value)
