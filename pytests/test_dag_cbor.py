import os

import libipld
import pytest

from conftest import load_cbor_data_fixtures, load_json_data_fixtures

_REAL_DATA_DIR = os.path.join(os.path.dirname(__file__), '..', 'data')
_ROUNDTRIP_DATA_DIR = os.path.join(_REAL_DATA_DIR, 'roundtrip')
_FIXTURES_DATA_DIR = os.path.join(_REAL_DATA_DIR, 'fixtures')
_TORTURE_CIDS_DAG_CBOR_PATH = os.path.join(_REAL_DATA_DIR, 'torture_cids.dagcbor')
_TORTURE_NESTED_LISTS_DAG_CBOR_PATH = os.path.join(_REAL_DATA_DIR, 'torture_nested_lists.dagcbor')
_TORTURE_NESTED_MAPS_DAG_CBOR_PATH = os.path.join(_REAL_DATA_DIR, 'torture_nested_maps.dagcbor')


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

    assert 'Map keys must be sorted' in str(exc_info.value)


def test_dag_cbor_decode_wrong_keys_order_duplicate_keys_error() -> None:
    with pytest.raises(ValueError) as exc_info:
        # {"abc": 1, "abd: 2", "abc": 1}
        libipld.decode_dag_cbor(bytes.fromhex('A3636162630163616264026361626301'))

    assert 'Map keys must be sorted' in str(exc_info.value)


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


@pytest.mark.benchmark_main
@pytest.mark.parametrize('data', load_json_data_fixtures(_REAL_DATA_DIR), ids=lambda data: data[0])
def test_dag_cbor_encode_real_data(benchmark, data) -> None:
    _dag_cbor_encode(benchmark, data)


@pytest.mark.benchmark_main
@pytest.mark.parametrize('data', load_json_data_fixtures(_REAL_DATA_DIR), ids=lambda data: data[0])
def test_dag_cbor_decode_real_data(benchmark, data) -> None:
    _dag_cbor_roundtrip(benchmark, data)


@pytest.mark.parametrize('data', load_cbor_data_fixtures(_FIXTURES_DATA_DIR), ids=lambda data: data[0])
def test_dag_cbor_decode_fixtures(benchmark, data) -> None:
    _dag_cbor_decode(benchmark, data)


@pytest.mark.benchmark_main
def test_dag_cbor_decode_torture_cids(benchmark) -> None:
    dag_cbor = open(_TORTURE_CIDS_DAG_CBOR_PATH, 'rb').read()
    benchmark(libipld.decode_dag_cbor, dag_cbor)


def test_recursion_limit_exceed_on_nested_lists() -> None:
    dag_cbor = open(_TORTURE_NESTED_LISTS_DAG_CBOR_PATH, 'rb').read()
    with pytest.raises(RecursionError) as exc_info:
        libipld.decode_dag_cbor(dag_cbor)

    assert 'in DAG-CBOR decoding' in str(exc_info.value)


def test_recursion_limit_exceed_on_nested_maps() -> None:
    dag_cbor = open(_TORTURE_NESTED_MAPS_DAG_CBOR_PATH, 'rb').read()
    with pytest.raises(RecursionError) as exc_info:
        libipld.decode_dag_cbor(dag_cbor)

    assert 'in DAG-CBOR decoding' in str(exc_info.value)


def test_dag_cbor_decode_largest_unsigned_int_roundtrip() -> None:
    data = bytes.fromhex('1bffffffffffffffff')

    decoded_result = libipld.decode_dag_cbor(data)
    assert decoded_result == 2**64 - 1

    encoded_result = libipld.encode_dag_cbor(decoded_result)
    assert encoded_result == data


def test_dag_cbor_decode_smallest_negative_int_roundtrip() -> None:
    data = bytes.fromhex('3bffffffffffffffff')

    decoded_result = libipld.decode_dag_cbor(data)
    assert decoded_result == -(2**64)

    encoded_result = libipld.encode_dag_cbor(decoded_result)
    assert encoded_result == data


def test_dag_cbor_decode_invalid_utf8() -> None:
    with pytest.raises(ValueError) as exc_info:
        libipld.decode_dag_cbor(bytes.fromhex('62c328'))


    assert 'utf-8' in str(exc_info.value)


def test_dab_cbor_decode_map_int_key() -> None:
    dag_cbor = bytes.fromhex('a10000')
    with pytest.raises(ValueError) as exc_info:
        libipld.decode_dag_cbor(dag_cbor)

    assert 'Map keys must be strings' in str(exc_info.value)


def test_dab_cbor_encode_map_int_key() -> None:
    obj = {0: 'value'}
    with pytest.raises(ValueError) as exc_info:
        libipld.encode_dag_cbor(obj)

    assert 'Map keys must be strings' in str(exc_info.value)


def test_dag_cbor_decode_nan_f64_error() -> None:
    # fb7ff8000000000000 - IEEE 754 double precision NaN
    with pytest.raises(ValueError) as exc_info:
        libipld.decode_dag_cbor(bytes.fromhex('fb7ff8000000000000'))

    assert 'number out of range for f64' in str(exc_info.value).lower()


def test_dag_cbor_decode_positive_infinity_f64_error() -> None:
    # fb7ff0000000000000 - IEEE 754 double precision positive infinity
    with pytest.raises(ValueError) as exc_info:
        libipld.decode_dag_cbor(bytes.fromhex('fb7ff0000000000000'))

    assert 'number out of range for f64' in str(exc_info.value).lower()


def test_dag_cbor_decode_negative_infinity_f64_error() -> None:
    # fbfff0000000000000 - IEEE 754 double precision negative infinity
    with pytest.raises(ValueError) as exc_info:
        libipld.decode_dag_cbor(bytes.fromhex('fbfff0000000000000'))

    assert 'number out of range for f64' in str(exc_info.value).lower()


def test_dag_cbor_decode_nan_f32_error() -> None:
    # fa7fc00000 - IEEE 754 single precision NaN
    with pytest.raises(ValueError) as exc_info:
        libipld.decode_dag_cbor(bytes.fromhex('fa7fc00000'))

    assert 'number out of range for f32' in str(exc_info.value).lower()


def test_dag_cbor_decode_positive_infinity_f32_error() -> None:
    # fa7f800000 - IEEE 754 single precision positive infinity
    with pytest.raises(ValueError) as exc_info:
        libipld.decode_dag_cbor(bytes.fromhex('fa7f800000'))

    assert 'number out of range for f32' in str(exc_info.value).lower()


def test_dag_cbor_decode_negative_infinity_f32_error() -> None:
    # faff800000 - IEEE 754 single precision negative infinity
    with pytest.raises(ValueError) as exc_info:
        libipld.decode_dag_cbor(bytes.fromhex('faff800000'))

    assert 'number out of range for f32' in str(exc_info.value).lower()


def test_dag_cbor_decode_cbor_sequence_error() -> None:
    # 0000 - two CBOR zeros (CBOR sequence), invalid in DAG-CBOR
    with pytest.raises(ValueError) as exc_info:
        libipld.decode_dag_cbor(bytes.fromhex('0000'))

    assert 'multiple objects' in str(exc_info.value).lower()


def test_decode_dag_cbor_multi() -> None:
    # 0000 - two CBOR zeros (CBOR sequence), valid only for decode_dag_cbor_multi
    dag_cbor = bytes.fromhex('0000')
    decoded = libipld.decode_dag_cbor_multi(dag_cbor)

    assert isinstance(decoded, list)
    assert len(decoded) == 2
    assert decoded[0] == 0
    assert decoded[1] == 0


def test_encode_tag_positive_bignum() -> None:
    bignum = 18446744073709551616

    with pytest.raises(ValueError) as exc_info:
         libipld.encode_dag_cbor(bignum)

    assert 'number out of range' in str(exc_info.value).lower()


def test_encode_tag_negative_bignum() -> None:
    bignum = -18446744073709551617

    with pytest.raises(ValueError) as exc_info:
         libipld.encode_dag_cbor(bignum)

    assert 'number out of range' in str(exc_info.value).lower()
