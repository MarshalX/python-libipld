import os
from pathlib import Path

import libipld
import pytest
from conftest import cbor_link, load_cbor_data_fixtures

_REAL_DATA_DIR = os.path.join(os.path.dirname(__file__), '..', 'data')
_FIXTURES_DATA_DIR = os.path.join(_REAL_DATA_DIR, 'fixtures')
_TORTURE_CIDS_DAG_CBOR_PATH = os.path.join(_REAL_DATA_DIR, 'torture_cids.dagcbor')
_TORTURE_NESTED_LISTS_DAG_CBOR_PATH = os.path.join(_REAL_DATA_DIR, 'torture_nested_lists.dagcbor')
_TORTURE_NESTED_MAPS_DAG_CBOR_PATH = os.path.join(_REAL_DATA_DIR, 'torture_nested_maps.dagcbor')

_DIGEST = '5891b5b522d5df086d0ff0b110fbd9d21bb4fc7163af34d08286a2e846f6be03'
_CID_V1 = '01551220' + _DIGEST
_CID_V0 = '1220' + _DIGEST
_NON_MINIMAL = 'Non-minimal header argument'

_CANONICAL_REJECTS = [
    ('uint-1-byte-0', '1801', _NON_MINIMAL),
    ('uint-1-byte-23', '1817', _NON_MINIMAL),
    ('uint-2-byte', '1900ff', _NON_MINIMAL),
    ('uint-4-byte', '1a0000ffff', _NON_MINIMAL),
    ('uint-8-byte', '1b00000000ffffffff', _NON_MINIMAL),
    ('nint-1-byte', '3800', _NON_MINIMAL),
    ('nint-2-byte', '3900ff', _NON_MINIMAL),
    ('text-1-byte', '780161', _NON_MINIMAL),
    ('text-2-byte', '79000161', _NON_MINIMAL),
    ('text-4-byte', '7a0000000161', _NON_MINIMAL),
    ('text-8-byte', '7b000000000000000161', _NON_MINIMAL),
    ('empty-text-1-byte', '7800', _NON_MINIMAL),
    ('bytes-1-byte', '580161', _NON_MINIMAL),
    ('array-1-byte', '980101', _NON_MINIMAL),
    ('array-8-byte', '9b000000000000000101', _NON_MINIMAL),
    ('map-1-byte', 'b801616101', _NON_MINIMAL),
    ('map-key-1-byte', 'a178016101', _NON_MINIMAL),
    ('tag-2-byte', 'd9002a' + cbor_link(_CID_V1)[4:], _NON_MINIMAL),
    ('tag-4-byte', 'da0000002a' + cbor_link(_CID_V1)[4:], _NON_MINIMAL),
    ('tag-8-byte', 'db000000000000002a' + cbor_link(_CID_V1)[4:], _NON_MINIMAL),
    ('link-length-2-byte', 'd82a59002500' + _CID_V1, _NON_MINIMAL),
    ('f32', 'fa3fc00000', 'Floats must be 64-bit'),
    ('f32-zero', 'fa00000000', 'Floats must be 64-bit'),
    ('nested-array', '81811801', _NON_MINIMAL),
    ('nested-map-value', 'a161611801', _NON_MINIMAL),
    ('nested-map-key', 'a16161a178016101', _NON_MINIMAL),
    ('array-after-links', '83' + cbor_link(_CID_V1) * 2 + '1801', _NON_MINIMAL),
]


@pytest.mark.parametrize('case', _CANONICAL_REJECTS, ids=lambda case: case[0])
def test_dag_cbor_decode_canonical_rejects(case) -> None:
    _, data, detail = case
    libipld.decode_dag_cbor(bytes.fromhex(data))

    with pytest.raises(ValueError) as exc_info:
        libipld.decode_dag_cbor(bytes.fromhex(data), canonical=True)

    assert str(exc_info.value) == f'Failed to decode DAG-CBOR. {detail}'


_CANONICAL_ACCEPTS = [
    ('uint-23', '17'),
    ('uint-24', '1818'),
    ('uint-255', '18ff'),
    ('uint-256', '190100'),
    ('uint-65535', '19ffff'),
    ('uint-65536', '1a00010000'),
    ('uint-2^32-1', '1affffffff'),
    ('uint-2^32', '1b0000000100000000'),
    ('uint-2^64-1', '1bffffffffffffffff'),
    ('nint-1', '20'),
    ('nint-24', '37'),
    ('nint-25', '3818'),
    ('nint-256', '38ff'),
    ('nint-257', '390100'),
    ('nint-2^64', '3bffffffffffffffff'),
    ('text-23', '77' + '61' * 23),
    ('text-24', '7818' + '61' * 24),
    ('text-255', '78ff' + '61' * 255),
    ('text-256', '790100' + '61' * 256),
    ('f64', 'fb3ff8000000000000'),
    ('f64-zero', 'fb0000000000000000'),
    ('f64-integral', 'fb3ff0000000000000'),
    ('f64-subnormal', 'fb0000000000000001'),
    ('cid-v1', cbor_link(_CID_V1)),
    ('cid-v0', cbor_link(_CID_V0)),
    ('cid-identity-empty-digest', cbor_link('01710000')),
    ('cid-digest-64', cbor_link('01711340' + 'ab' * 64)),
    ('cid-2-byte-codec', cbor_link('0180011220' + _DIGEST)),
    ('cid-9-byte-codec', cbor_link('018080808080808080011220' + _DIGEST)),
    ('cid-10-byte-codec', cbor_link('01808080808080808080011220' + _DIGEST)),
    ('nested', 'a26161818201f56162a1616180'),
]


@pytest.mark.parametrize('case', _CANONICAL_ACCEPTS, ids=lambda case: case[0])
def test_dag_cbor_decode_canonical_accepts(case) -> None:
    _, data = case
    data = bytes.fromhex(data)

    decoded = libipld.decode_dag_cbor(data, canonical=True)

    assert decoded == libipld.decode_dag_cbor(data)
    assert libipld.encode_dag_cbor(decoded) == data


@pytest.mark.parametrize(
    'data',
    [
        'a2616201616101',  # unsorted keys
        'a2616101616102',  # duplicate keys
        'a10101',  # non-string key
        'f93e00',  # f16
        'fa7fc00000',  # f32 NaN
        'fa7f800000',  # f32 Infinity
        'fa80000000',  # f32 -0.0
        'fb7ff8000000000000',  # f64 NaN
        'fb8000000000000000',  # f64 -0.0
        'd82b4100',  # tag 43
        'd9002b4100',  # tag 43 with a 2-byte head
        '0000',  # trailing data
        '62c328',  # invalid UTF-8
        'd82a6161',  # tag 42 on text
        'd82a4100',  # CID prefix only
        'd82a40',  # empty CID
        'd82a5824' + _CID_V1,  # CID without the 0x00 prefix
        'd82a450001711220',  # truncated CID
        'd82a4e000171' + 'ff' * 10 + '01',  # CID with an 11-byte varint
        cbor_link(_CID_V1 + '01'),  # bytes after the CID
        cbor_link('01ffffffffffffffffff021220' + _DIGEST),  # CID codec over 64 bits
        cbor_link('01711241' + 'ab' * 65),  # CID with a 65-byte digest
        '9a40000000',  # array claiming 2**30 items
        '83' + cbor_link(_CID_V1) * 2 + '6361',  # truncated after two links
    ],
)
def test_dag_cbor_decode_canonical_keeps_default_errors(data) -> None:
    data = bytes.fromhex(data)

    with pytest.raises(ValueError) as default_exc_info:
        libipld.decode_dag_cbor(data)
    with pytest.raises(ValueError) as exc_info:
        libipld.decode_dag_cbor(data, canonical=True)

    assert str(exc_info.value) == str(default_exc_info.value)


def test_dag_cbor_decode_canonical_first_error_wins() -> None:
    # bytes with a non-minimal 2-byte length, then truncated
    data = bytes.fromhex('5900ff00')

    with pytest.raises(ValueError) as exc_info:
        libipld.decode_dag_cbor(data)
    assert 'end of data' in str(exc_info.value)

    with pytest.raises(ValueError) as exc_info:
        libipld.decode_dag_cbor(data, canonical=True)
    assert _NON_MINIMAL in str(exc_info.value)


def test_dag_cbor_decode_canonical_false() -> None:
    data = bytes.fromhex('1801')

    assert libipld.decode_dag_cbor(data, canonical=False) == libipld.decode_dag_cbor(data) == 1


@pytest.mark.parametrize('data', load_cbor_data_fixtures(_FIXTURES_DATA_DIR), ids=lambda data: data[0])
def test_dag_cbor_decode_canonical_fixtures(data) -> None:
    _, fixture = data
    dag_cbor = fixture.get('dag-cbor')

    assert libipld.decode_dag_cbor(dag_cbor, canonical=True) == libipld.decode_dag_cbor(dag_cbor)


def test_dag_cbor_decode_canonical_torture_cids() -> None:
    dag_cbor = Path(_TORTURE_CIDS_DAG_CBOR_PATH).read_bytes()

    assert libipld.decode_dag_cbor(dag_cbor, canonical=True) == libipld.decode_dag_cbor(dag_cbor)


@pytest.mark.parametrize('path', [_TORTURE_NESTED_LISTS_DAG_CBOR_PATH, _TORTURE_NESTED_MAPS_DAG_CBOR_PATH])
def test_recursion_limit_exceed_on_decode_canonical(path) -> None:
    dag_cbor = Path(path).read_bytes()
    with pytest.raises(RecursionError) as exc_info:
        libipld.decode_dag_cbor(dag_cbor, canonical=True)

    assert 'in DAG-CBOR decoding' in str(exc_info.value)


def test_dag_cbor_encode_duplicate_keys_is_not_canonical() -> None:
    class Key(str):
        def __eq__(self, other: object) -> bool:
            return False

        def __hash__(self) -> int:
            return id(self)

    encoded = libipld.encode_dag_cbor({Key('a'): 1, Key('a'): 2})

    assert encoded.hex() == 'a2616101616102'
    with pytest.raises(ValueError) as exc_info:
        libipld.decode_dag_cbor(encoded, canonical=True)

    assert 'Map keys must be sorted' in str(exc_info.value)
