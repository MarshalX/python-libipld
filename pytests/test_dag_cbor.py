import math
import os
import sys
from pathlib import Path

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
    dag_cbor = Path(_TORTURE_CIDS_DAG_CBOR_PATH).read_bytes()
    benchmark(libipld.decode_dag_cbor, dag_cbor)


def test_recursion_limit_exceed_on_nested_lists() -> None:
    dag_cbor = Path(_TORTURE_NESTED_LISTS_DAG_CBOR_PATH).read_bytes()
    with pytest.raises(RecursionError) as exc_info:
        libipld.decode_dag_cbor(dag_cbor)

    assert 'in DAG-CBOR decoding' in str(exc_info.value)


def test_recursion_limit_exceed_on_nested_maps() -> None:
    dag_cbor = Path(_TORTURE_NESTED_MAPS_DAG_CBOR_PATH).read_bytes()
    with pytest.raises(RecursionError) as exc_info:
        libipld.decode_dag_cbor(dag_cbor)

    assert 'in DAG-CBOR decoding' in str(exc_info.value)


def _nest(kind: str, depth: int):
    obj = [] if kind == 'list' else {}
    for _ in range(depth):
        obj = [obj] if kind == 'list' else {'a': obj}
    return obj


@pytest.mark.parametrize('kind', ['list', 'dict'])
def test_recursion_limit_exceed_on_encode(kind) -> None:
    obj = _nest(kind, sys.getrecursionlimit() + 100)
    with pytest.raises(RecursionError) as exc_info:
        libipld.encode_dag_cbor(obj)

    assert 'in DAG-CBOR encoding' in str(exc_info.value)


@pytest.mark.parametrize('kind', ['list', 'dict'])
def test_nested_within_recursion_limit_roundtrip(kind) -> None:
    obj = _nest(kind, sys.getrecursionlimit() // 2)
    assert libipld.decode_dag_cbor(libipld.encode_dag_cbor(obj)) == obj


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

    assert str(exc_info.value).startswith('Failed to decode DAG-CBOR. UnicodeDecodeError:')
    assert 'utf-8' in str(exc_info.value)


def test_dag_cbor_decode_lone_surrogate() -> None:
    text = '😍😍😍😍'.encode() + bytes.fromhex('eda0bd')
    assert len(text) == 19
    with pytest.raises(ValueError) as exc_info:
        libipld.decode_dag_cbor(bytes([0x60 | len(text)]) + text)

    msg = str(exc_info.value)
    assert msg.startswith('Failed to decode DAG-CBOR. UnicodeDecodeError:')
    assert 'position 16' in msg


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


def test_dag_cbor_decode_negative_zero_f64_error() -> None:
    # fb8000000000000000 - IEEE 754 double precision negative zero, forbidden in DAG-CBOR
    with pytest.raises(ValueError) as exc_info:
        libipld.decode_dag_cbor(bytes.fromhex('fb8000000000000000'))

    assert 'negative zero' in str(exc_info.value).lower()


def test_dag_cbor_decode_negative_zero_f32_error() -> None:
    # fa80000000 - IEEE 754 single precision negative zero, forbidden in DAG-CBOR
    with pytest.raises(ValueError) as exc_info:
        libipld.decode_dag_cbor(bytes.fromhex('fa80000000'))

    assert 'negative zero' in str(exc_info.value).lower()


def test_dag_cbor_decode_positive_zero_f64() -> None:
    decoded = libipld.decode_dag_cbor(bytes.fromhex('fb0000000000000000'))

    assert decoded == 0.0
    assert math.copysign(1.0, decoded) == 1.0


def test_dag_cbor_encode_negative_zero_as_positive_zero() -> None:
    # -0.0 equals 0.0 and is forbidden in DAG-CBOR, so it is encoded as 0.0
    assert libipld.encode_dag_cbor(-0.0) == bytes.fromhex('fb0000000000000000')
    assert libipld.encode_dag_cbor(-0.0) == libipld.encode_dag_cbor(0.0)


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


def test_roundtrip_valid_cid_with_short_tag() -> None:
    encoded_hex = 'd82a582500015512205891b5b522d5df086d0ff0b110fbd9d21bb4fc7163af34d08286a2e846f6be03'
    encoded_bytes = bytes.fromhex(encoded_hex)

    decoded = libipld.decode_dag_cbor(encoded_bytes)
    encoded = libipld.encode_dag_cbor(decoded)

    assert encoded == encoded_bytes


def test_dag_cbor_decode_array_length_exceeds_data_error() -> None:
    # 9b0000000040000000 - array claiming 2**30 elements with no payload
    with pytest.raises(ValueError) as exc_info:
        libipld.decode_dag_cbor(bytes.fromhex('9b0000000040000000'))

    assert 'Array length exceeds remaining data' in str(exc_info.value)


def test_dag_cbor_decode_map_length_exceeds_data_error() -> None:
    # bb0000000040000000 - map claiming 2**30 entries with no payload
    with pytest.raises(ValueError) as exc_info:
        libipld.decode_dag_cbor(bytes.fromhex('bb0000000040000000'))

    assert 'Map length exceeds remaining data' in str(exc_info.value)


def test_dag_cbor_decode_error_mid_array() -> None:
    # 83010263616263 - [1, 2, "abc"] truncated after the second element's header
    with pytest.raises(ValueError):
        libipld.decode_dag_cbor(bytes.fromhex('8301026361'))


@pytest.mark.parametrize('length', [*range(70), 255, 256, 257, 65535, 65536, 65537])
def test_dag_cbor_roundtrip_string_and_bytes_lengths(length) -> None:
    text = ('abcdefghij' * (length // 10 + 1))[:length]
    obj = {'b': text.encode(), 's': text, text: length, 'l': [0] * min(length, 300)}

    encoded = libipld.encode_dag_cbor(obj)
    assert obj == libipld.decode_dag_cbor(encoded)

    if length < 24:
        head = bytes([0x60 | length])
    elif length < 256:
        head = bytes([0x78, length])
    elif length < 65536:
        head = b'\x79' + length.to_bytes(2, 'big')
    else:
        head = b'\x7a' + length.to_bytes(4, 'big')
    assert libipld.encode_dag_cbor(text) == head + text.encode()


@pytest.mark.parametrize(
    ('value', 'expected'),
    [
        (0, '00'),
        (23, '17'),
        (24, '1818'),
        (255, '18ff'),
        (256, '190100'),
        (65535, '19ffff'),
        (65536, '1a00010000'),
        (2**32 - 1, '1affffffff'),
        (2**32, '1b0000000100000000'),
        (2**64 - 1, '1bffffffffffffffff'),
        (-1, '20'),
        (-24, '37'),
        (-25, '3818'),
        (-(2**64), '3bffffffffffffffff'),
        (1.5, 'fb3ff8000000000000'),
        (True, 'f5'),
        (False, 'f4'),
        (None, 'f6'),
        (b'\x01\x02', '420102'),
        ([], '80'),
        ({}, 'a0'),
    ],
)
def test_dag_cbor_encode_canonical_heads(value, expected) -> None:
    encoded = libipld.encode_dag_cbor(value)
    assert encoded.hex() == expected
    assert libipld.decode_dag_cbor(encoded) == value


def test_dag_cbor_encode_map_key_order() -> None:
    keys = [
        '',
        'b',
        'a',
        'zz',
        'aaaaaaaa',
        'aaaaaaab',
        'aaaaaaaaa',
        'aaaaaaaab',
        'baaaaaaaa',
        'aaaaaaaa\x00',
        'prefix__0002',
        'prefix__0001',
        'prefix__0003',
        'é',
        'zé',
        'ab',
        '\x7f',
        'Ā',
    ]
    expected = sorted(keys, key=lambda k: (len(k.encode()), k.encode()))

    for ordering in (keys, keys[::-1], expected):
        encoded = libipld.encode_dag_cbor({k: i for i, k in enumerate(ordering)})
        # decode rejects unsorted keys and keeps their order
        assert list(libipld.decode_dag_cbor(encoded)) == expected


def test_dag_cbor_decode_many_distinct_keys() -> None:
    # more distinct keys than the key cache holds
    obj = [{f'key{i}': i, f'k{i}': None} for i in range(6000)]
    encoded = libipld.encode_dag_cbor(obj)

    assert libipld.decode_dag_cbor(encoded) == obj
    assert libipld.decode_dag_cbor(encoded) == obj


def test_dag_cbor_cid_roundtrip_shapes() -> None:
    cids = [
        bytes.fromhex('01711220') + bytes(range(32)),  # v1 dag-cbor sha2-256
        bytes.fromhex('01551220') + bytes(range(32)),  # v1 raw sha2-256
        bytes.fromhex('01701340') + bytes(range(64)),  # v1 dag-pb sha2-512
        bytes.fromhex('0171000401020304'),  # v1 identity, 4-byte digest
        bytes.fromhex('1220') + bytes(range(32)),  # v0
    ]
    for cid in cids:
        encoded = libipld.encode_dag_cbor({'c': cid})
        assert encoded[3:5] == b'\xd8\x2a'  # tag 42
        assert libipld.decode_dag_cbor(encoded) == {'c': cid}

    # truncated digest: plain bytes, not a CID
    not_cid = bytes.fromhex('01711220') + bytes(31)
    assert libipld.encode_dag_cbor(not_cid)[0] == 0x58
