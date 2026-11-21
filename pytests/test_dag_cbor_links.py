import collections
import os
import random
from pathlib import Path

import libipld
import pytest
from conftest import cbor_link, load_cbor_data_fixtures, load_json_data_fixtures
from libipld import CID

_REAL_DATA_DIR = os.path.join(os.path.dirname(__file__), '..', 'data')
_ROUNDTRIP_DATA_DIR = os.path.join(_REAL_DATA_DIR, 'roundtrip')
_FIXTURES_DATA_DIR = os.path.join(_REAL_DATA_DIR, 'fixtures')
_TORTURE_CIDS_DAG_CBOR_PATH = os.path.join(_REAL_DATA_DIR, 'torture_cids.dagcbor')

_DIGEST = '5891b5b522d5df086d0ff0b110fbd9d21bb4fc7163af34d08286a2e846f6be03'
_CID_V1 = '01551220' + _DIGEST
_CID_V0 = '1220' + _DIGEST
_CID_V1_BYTES = bytes.fromhex(_CID_V1)
_CID_V0_BYTES = bytes.fromhex(_CID_V0)

# {"a": [link v1, {"b": link v0}], "c": bytes holding a CID, "d": link v1}
_LINKS = (
    'a3616182' + cbor_link(_CID_V1) + 'a16162' + cbor_link(_CID_V0) + '61635824' + _CID_V1 + '6164' + cbor_link(_CID_V1)
)

_CANONICAL = pytest.mark.parametrize('canonical', [False, True], ids=['default', 'canonical'])

_TRAILING = 'Invalid CID: bytes after the CID'
_OVERFLOW = 'Invalid CID: varint over 64 bits'
_NOT_ONE_CID = [
    ('cid-v1-trailing-byte', cbor_link(_CID_V1 + '01'), _TRAILING),
    ('cid-v1-trailing-bytes', cbor_link(_CID_V1 + '0102'), _TRAILING),
    ('cid-v1-trailing-cid', cbor_link(_CID_V1 + _CID_V1), _TRAILING),
    ('cid-v0-trailing-byte', cbor_link(_CID_V0 + '01'), _TRAILING),
    ('cid-codec-overflow', cbor_link('01ffffffffffffffffff021220' + _DIGEST), _OVERFLOW),
    ('cid-hash-code-overflow', cbor_link('01719280808080808080800220' + _DIGEST), _OVERFLOW),
    ('cid-digest-size-overflow', cbor_link('017112a0808080808080808002' + _DIGEST), _OVERFLOW),
]


@_CANONICAL
@pytest.mark.parametrize('case', _NOT_ONE_CID, ids=lambda case: case[0])
def test_dag_cbor_decode_rejects_link_that_is_not_one_cid(case, canonical) -> None:
    _, data, detail = case
    with pytest.raises(ValueError) as exc_info:
        libipld.decode_dag_cbor(bytes.fromhex(data), canonical=canonical)

    assert str(exc_info.value) == f'Failed to decode DAG-CBOR. {detail}'


@_CANONICAL
@pytest.mark.parametrize('case', _NOT_ONE_CID, ids=lambda case: case[0])
def test_dag_cbor_decode_multi_rejects_link_that_is_not_one_cid(case, canonical) -> None:
    _, data, detail = case
    with pytest.raises(ValueError) as exc_info:
        libipld.decode_dag_cbor_multi(bytes.fromhex('01' + data), canonical=canonical)

    assert str(exc_info.value) == f'Failed to decode DAG-CBOR. {detail}'


@_CANONICAL
@pytest.mark.parametrize(
    'data',
    [
        'd82a5824' + _CID_V1,  # no 0x00 prefix
        'd82a5825' + '01' + _CID_V1,  # 0x01 prefix
        'd82a4100',  # prefix only
        'd82a40',  # empty
        'd82a6161',  # tag 42 on text
        cbor_link('02551220' + _DIGEST),  # CID version 2
        cbor_link('01f1001220' + _DIGEST),  # non-minimal codec varint
    ],
)
def test_dag_cbor_decode_invalid_link(data, canonical) -> None:
    with pytest.raises(ValueError) as exc_info:
        libipld.decode_dag_cbor(bytes.fromhex(data), canonical=canonical)

    assert str(exc_info.value) == 'Failed to decode DAG-CBOR. Invalid CID'


@_CANONICAL
def test_dag_cbor_decode_links(canonical) -> None:
    decoded = libipld.decode_dag_cbor(bytes.fromhex(_LINKS), canonical=canonical)

    assert decoded == {
        'a': [CID(_CID_V1_BYTES), {'b': CID(_CID_V0_BYTES)}],
        'c': _CID_V1_BYTES,
        'd': CID(_CID_V1_BYTES),
    }
    assert type(decoded['a'][0]) is CID
    assert type(decoded['c']) is bytes


_LONGEST_CID = '01' + '80808080808080808001' * 2 + '40' + 'cd' * 64


@_CANONICAL
def test_dag_cbor_longest_cid_round_trip(canonical) -> None:
    data = bytes.fromhex('a16161' + cbor_link(_LONGEST_CID))

    decoded = libipld.decode_dag_cbor(data, canonical=canonical)

    assert decoded == {'a': CID(bytes.fromhex(_LONGEST_CID))}
    assert libipld.encode_dag_cbor(decoded) == data


def test_dag_cbor_decode_link_top_level() -> None:
    assert libipld.decode_dag_cbor(bytes.fromhex(cbor_link(_CID_V1))) == CID(_CID_V1_BYTES)


@pytest.mark.parametrize(
    'cid',
    [
        '01711e20' + 'ab' * 32,  # blake3
        '0171b0e40220' + 'ab' * 32,  # unknown multihash code
        '01f0e0031220' + _DIGEST,  # unknown codec
        '01710000',  # identity, empty digest
    ],
    ids=['blake3', 'unknown-hash', 'unknown-codec', 'identity'],
)
def test_dag_cbor_decode_any_codec_and_hash(cid) -> None:
    decoded = libipld.decode_dag_cbor(bytes.fromhex(cbor_link(cid)), canonical=True)

    assert bytes(decoded) == bytes.fromhex(cid)


def test_dag_cbor_encode_bytes_stay_bytes() -> None:
    class Bytes(bytes):
        pass

    for value in (_CID_V1_BYTES, _CID_V0_BYTES, b'\x01' * 32, _CID_V1_BYTES + b'\x01', Bytes(_CID_V1_BYTES)):
        encoded = libipld.encode_dag_cbor({'a': value})

        assert encoded[3] == 0x58
        assert libipld.decode_dag_cbor(encoded, canonical=True) == {'a': value}


def test_dag_cbor_encode_cid_str_stays_text() -> None:
    cid = 'bafyreig7jbijxpn4lfhvnvyuwf5u5jyhd7begxwyiqe7ingwxycjdqjjoa'

    assert libipld.decode_dag_cbor(libipld.encode_dag_cbor(cid)) == cid


@pytest.mark.parametrize(
    'value,expected',
    [
        (CID(_CID_V1_BYTES), cbor_link(_CID_V1)),
        (CID(_CID_V0_BYTES), cbor_link(_CID_V0)),
        (CID(bytes.fromhex('01710000')), cbor_link('01710000')),
        (CID(bytes.fromhex('01711340' + 'ab' * 64)), cbor_link('01711340' + 'ab' * 64)),
        ([CID(_CID_V1_BYTES), 1], '82' + cbor_link(_CID_V1) + '01'),
        ({'a': CID(_CID_V0_BYTES)}, 'a16161' + cbor_link(_CID_V0)),
        ((CID(_CID_V1_BYTES), (1, 2)), '82' + cbor_link(_CID_V1) + '820102'),
    ],
    ids=['v1', 'v0', 'identity', 'digest-64', 'list', 'map', 'tuple'],
)
def test_dag_cbor_encode_links(value, expected) -> None:
    assert libipld.encode_dag_cbor(value).hex() == expected


@pytest.mark.parametrize(
    'value,expected',
    [
        ((), '80'),
        ((1, 2), '820102'),
        ([(1,), ((), [])], '828101828080'),
        ({'a': (None, True)}, 'a1616182f6f5'),
    ],
)
def test_dag_cbor_encode_tuple(value, expected) -> None:
    assert libipld.encode_dag_cbor(value).hex() == expected


@pytest.mark.parametrize(
    'value',
    [
        {'a': [set()]},
        bytearray(b'a'),
        1j,
        object(),
        libipld.CID(_CID_V1_BYTES).hash,
        collections.namedtuple('point', ['x', 'y'])(1, 2),
    ],
)
def test_dag_cbor_encode_unknown_type_error(value) -> None:
    with pytest.raises(ValueError) as exc_info:
        libipld.encode_dag_cbor(value)

    assert str(exc_info.value) == 'Failed to encode DAG-CBOR. Unknown tag'


def test_dag_cbor_encode_cid_map_key_error() -> None:
    with pytest.raises(ValueError):
        libipld.encode_dag_cbor({CID(_CID_V1_BYTES): 1})


def _round_trip_inputs() -> list:
    inputs = [(name, fixture['dag-cbor']) for name, fixture in load_cbor_data_fixtures(_FIXTURES_DATA_DIR)]
    for name, obj in load_json_data_fixtures(_ROUNDTRIP_DATA_DIR) + load_json_data_fixtures(_REAL_DATA_DIR):
        inputs.append((name, libipld.encode_dag_cbor(obj)))
    inputs.append(('torture_cids', Path(_TORTURE_CIDS_DAG_CBOR_PATH).read_bytes()))
    inputs.append(('links', bytes.fromhex(_LINKS)))
    return inputs


@_CANONICAL
@pytest.mark.parametrize('data', _round_trip_inputs(), ids=lambda data: data[0])
def test_dag_cbor_round_trip_is_byte_exact(data, canonical) -> None:
    _, dag_cbor = data

    assert libipld.encode_dag_cbor(libipld.decode_dag_cbor(dag_cbor, canonical=canonical)) == dag_cbor


def _random_bytes(rng: random.Random, size: int) -> bytes:
    return bytes(rng.randrange(256) for _ in range(size))


def _random_cid(rng: random.Random) -> bytes:
    if rng.random() < 0.2:
        return b'\x12\x20' + _random_bytes(rng, 32)
    digest = _random_bytes(rng, rng.choice([0, 1, 20, 32, 64]))
    codec = rng.choice([0x55, 0x70, 0x71, 0x0129, 2**63])
    varints = b''
    for value in (codec, rng.choice([0x12, 0x1E]), len(digest)):
        while value >= 0x80:
            varints += bytes([value & 0x7F | 0x80])
            value >>= 7
        varints += bytes([value])
    return b'\x01' + varints + digest


def _random_value(rng: random.Random, depth: int = 0):
    kind = rng.randrange(13 if depth < 4 else 9)
    if kind == 0:
        return rng.choice([None, True, False])
    if kind == 1:
        return rng.choice([0, 23, 24, 255, 256, 2**32, 2**64 - 1, -1, -24, -25, -(2**64)])
    if kind == 2:
        return rng.randint(-(2**64), 2**64 - 1)
    if kind == 3:
        return rng.choice([0.5, 1.0, 5e-324, -1.5e300, rng.uniform(-1e6, 1e6)])
    if kind == 4:
        return ''.join(rng.choice('abé中\U0001f600') for _ in range(rng.choice([0, 1, 23, 24, 300])))
    if kind == 5:
        return _random_bytes(rng, rng.choice([0, 1, 32, 300]))
    if kind == 6:
        return _random_cid(rng)
    if kind in (7, 8):
        return CID(_random_cid(rng))
    if kind in (9, 10):
        items = [_random_value(rng, depth + 1) for _ in range(rng.randrange(5))]
        return items if kind == 9 else tuple(items)
    return {
        rng.choice(['', 'a', 'b', 'ab', 'ba', 'abc', 'x' * 24, 'é']): _random_value(rng, depth + 1)
        for _ in range(rng.randrange(6))
    }


def _without_tuples(value):
    if isinstance(value, (list, tuple)):
        return [_without_tuples(item) for item in value]
    if isinstance(value, dict):
        return {key: _without_tuples(item) for key, item in value.items()}
    return value


@_CANONICAL
@pytest.mark.parametrize('seed', range(20))
def test_dag_cbor_round_trip_random(seed, canonical) -> None:
    rng = random.Random(seed)
    for _ in range(50):
        value = _random_value(rng)

        encoded = libipld.encode_dag_cbor(value)
        decoded = libipld.decode_dag_cbor(encoded, canonical=canonical)

        assert decoded == _without_tuples(value)
        assert libipld.encode_dag_cbor(decoded) == encoded
