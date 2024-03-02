import os

import libipld
import pytest

from conftest import load_cbor_data_fixtures, load_json_data_fixtures

_ROUNDTRIP_DATA_DIR = os.path.join(os.path.dirname(__file__), '..', 'data', 'roundtrip')
_REAL_DATA_DIR = os.path.join(os.path.dirname(__file__), '..', 'data')
_FIXTURES_DATA_DIR = os.path.join(os.path.dirname(__file__), '..', 'data', 'fixtures')
_CIDS_DAG_CBOR_PATH = os.path.join(os.path.dirname(__file__), '..', 'data', 'torture_cids.dag-cbor')


def _dag_cbor_encode(benchmark, data) -> None:
    _, obj = data

    encoded = benchmark(libipld.encode_dag_cbor, obj)

    assert isinstance(encoded, bytes)


def _dag_cbor_decode(benchmark, data) -> None:
    _, fixture = data
    dag_cbor = fixture.get('dag-cbor')

    benchmark(libipld.decode_dag_cbor, dag_cbor)


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


def test_dag_cbor_decode_wrong_keys_order_lexical_error() -> None:
    with pytest.raises(ValueError) as exc_info:
        # {"def": 1, "abc": 2} (same key lengths, wrong sort order)
        libipld.decode_dag_cbor(bytes.fromhex('a263646566016361626302'))

    assert 'Map keys must be sorted' in str(exc_info.value)


def test_dag_cbor_decode_wrong_keys_order_length_error() -> None:
    with pytest.raises(ValueError) as exc_info:
        # {"aaa": 1, "x": 2} (different key lengths, wrong sort order)
        libipld.decode_dag_cbor(bytes.fromhex('a26361616101617802'))

    assert 'Map keys must be sorted' in str(exc_info.value)


def test_dag_cbor_encode_wrong_keys_order_error() -> None:
    obj = {'aaa': 1, 'x': 2}
    obj2 = {'x': 2, 'aaa': 1}

    encoded = libipld.encode_dag_cbor(obj)
    encoded2 = libipld.encode_dag_cbor(obj2)

    assert encoded == encoded2
    assert b'\xa2ax\x02caaa\x01' == encoded
    assert b'\xa2caaa\x01ax\x02' != encoded
    assert b'\xa2ax\x02caaa\x01' == encoded2
    assert b'\xa2caaa\x01ax\x02' != encoded2


@pytest.mark.parametrize('data', load_json_data_fixtures(_ROUNDTRIP_DATA_DIR), ids=lambda data: data[0])
def test_dag_cbor_encode(benchmark, data) -> None:
    _dag_cbor_encode(benchmark, data)


@pytest.mark.parametrize('data', load_json_data_fixtures(_ROUNDTRIP_DATA_DIR), ids=lambda data: data[0])
def test_dag_cbor_decode(benchmark, data) -> None:
    _dag_cbor_roundtrip(benchmark, data)


@pytest.mark.parametrize('data', load_json_data_fixtures(_REAL_DATA_DIR), ids=lambda data: data[0])
def test_dag_cbor_encode_real_data(benchmark, data) -> None:
    _dag_cbor_encode(benchmark, data)


@pytest.mark.parametrize('data', load_json_data_fixtures(_REAL_DATA_DIR), ids=lambda data: data[0])
def test_dag_cbor_decode_real_data(benchmark, data) -> None:
    _dag_cbor_roundtrip(benchmark, data)


@pytest.mark.parametrize('data', load_cbor_data_fixtures(_FIXTURES_DATA_DIR), ids=lambda data: data[0])
def test_dag_cbor_decode_fixtures(benchmark, data) -> None:
    _dag_cbor_decode(benchmark, data)


def test_dag_cbor_decode_torture_cids(benchmark) -> None:
    dag_cbor = open(_CIDS_DAG_CBOR_PATH, 'rb').read()
    benchmark(libipld.decode_dag_cbor, dag_cbor)
