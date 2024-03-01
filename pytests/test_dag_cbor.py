import os

import libipld
import pytest

from conftest import load_data_fixtures

_ROUNDTRIP_DATA_DIR = os.path.join(os.path.dirname(__file__), '..', 'data', 'roundtrip')
_REAL_DATA_DIR = os.path.join(os.path.dirname(__file__), '..', 'data')


def _dag_cbor_encode(benchmark, data) -> None:
    _, obj = data

    encoded = benchmark(libipld.encode_dag_cbor, obj)

    assert isinstance(encoded, bytes)


def _dag_cbor_roundtrip(benchmark, data) -> None:
    _, obj = data

    encoded = libipld.encode_dag_cbor(obj)
    decoded = benchmark(libipld.decode_dag_cbor, encoded)

    assert obj == decoded, f'{obj} != {decoded}'


def test_dag_cbor_decode_duplicate_keys_error() -> None:
    with pytest.raises(ValueError) as exc_info:
        # {"abc": 1, "abc": 2}
        libipld.decode_dag_cbor(bytes.fromhex('a263616263016361626302'))

    assert 'Duplicate keys are not allowed' in str(exc_info.value)


def test_dag_cbor_decode_non_string_key_error() -> None:
    with pytest.raises(ValueError) as exc_info:
        # {1:2}
        libipld.decode_dag_cbor(bytes.fromhex('A10102'))

    assert 'Map keys must be strings' in str(exc_info.value)


@pytest.mark.parametrize('data', load_data_fixtures(_ROUNDTRIP_DATA_DIR), ids=lambda data: data[0])
def test_dag_cbor_encode(benchmark, data) -> None:
    _dag_cbor_encode(benchmark, data)


@pytest.mark.parametrize('data', load_data_fixtures(_ROUNDTRIP_DATA_DIR), ids=lambda data: data[0])
def test_dag_cbor_decode(benchmark, data) -> None:
    _dag_cbor_roundtrip(benchmark, data)


@pytest.mark.parametrize('data', load_data_fixtures(_REAL_DATA_DIR), ids=lambda data: data[0])
def test_dag_cbor_encode_real_data(benchmark, data) -> None:
    _dag_cbor_encode(benchmark, data)


@pytest.mark.parametrize('data', load_data_fixtures(_REAL_DATA_DIR), ids=lambda data: data[0])
def test_dag_cbor_decode_real_data(benchmark, data) -> None:
    _dag_cbor_roundtrip(benchmark, data)
