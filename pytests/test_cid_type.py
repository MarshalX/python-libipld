import copy
import pickle
import platform
import threading

import libipld
import pytest
from libipld import CID, Multihash

_V1 = 'bafyreie5737gdxlw5i64vzichcalba3z2v5n6icifvx5xytvske7mr3hpm'
_V0 = 'QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG'
_V1_BYTES = libipld.decode_multibase(_V1)[1]
_V0_BYTES = libipld.decode_multibase('z' + _V0)[1]


@pytest.mark.parametrize(
    'value',
    [
        _V1,
        _V1.upper(),
        _V1_BYTES,
        bytearray(_V1_BYTES),
        libipld.encode_multibase('z', _V1_BYTES),
        libipld.encode_multibase('u', _V1_BYTES),
    ],
    ids=['base32', 'base32upper', 'bytes', 'bytearray', 'base58btc', 'base64url'],
)
def test_cid_from_str_or_bytes(value) -> None:
    cid = CID(value)

    assert cid == CID.decode(value)
    assert str(cid) == cid.encode() == _V1
    assert bytes(cid) == _V1_BYTES
    assert (cid.version, cid.codec) == (1, 0x71)
    assert isinstance(cid.hash, Multihash)
    assert (cid.hash.code, cid.hash.size, cid.hash.digest) == (0x12, 32, _V1_BYTES[4:])
    assert cid.hash == CID(_V1).hash


def test_cid_v0() -> None:
    cid = CID(_V0)

    assert CID(_V0_BYTES) == cid
    assert str(cid) == _V0
    assert bytes(cid) == _V0_BYTES
    assert (cid.version, cid.codec, cid.hash.code, cid.hash.size) == (0, 0x70, 0x12, 32)


def test_cid_large_varints() -> None:
    raw = bytes.fromhex('01808080808080808080011e40') + b'\xab' * 64
    cid = CID(raw)

    assert (cid.version, cid.codec, cid.hash.code, cid.hash.size) == (1, 2**63, 0x1E, 64)
    assert CID(str(cid)) == cid


@pytest.mark.parametrize(
    'value,detail',
    [
        (_V1_BYTES + b'\x00', 'Invalid CID: bytes after the CID'),
        (libipld.encode_multibase('b', _V1_BYTES + b'\x00'), 'Invalid CID: bytes after the CID'),
        (bytes.fromhex('01719280808080808080800220') + bytes(32), 'Invalid CID: varint over 64 bits'),
        (b'\x02' + _V1_BYTES[1:], 'Invalid CID'),
    ],
)
def test_cid_invalid_detail(value, detail) -> None:
    with pytest.raises(ValueError) as exc_info:
        CID(value)

    assert str(exc_info.value) == f'Failed to decode CID. {detail}'


def test_cid_longest() -> None:
    raw = b'\x01' + bytes.fromhex('80808080808080808001') * 2 + b'\x40' + b'\xcd' * 64
    cid = CID(raw)

    assert len(raw) == 86
    assert bytes(cid) == raw
    assert (cid.version, cid.codec, cid.hash.code, cid.hash.size, cid.hash.digest) == (1, 2**63, 2**63, 64, raw[22:])
    assert CID(str(cid)) == cid
    assert libipld.encode_cid(cid) == libipld.encode_cid(raw) == str(cid)
    assert libipld.decode_cid(cid) == libipld.decode_cid(raw)
    assert libipld.decode_cid(cid)['codec'] == 2**63
    with pytest.raises(ValueError):
        CID(raw + b'\xcd')


@pytest.mark.parametrize('value', ['/ipfs/' + _V1, 'z' + _V0])
def test_cid_str_accepts_only_bare_forms(value) -> None:
    with pytest.raises(ValueError):
        CID(value)

    assert libipld.decode_cid(value) == libipld.decode_cid(CID(_V1 if _V1 in value else _V0))


def test_cid_from_cid_is_the_same_object() -> None:
    cid = CID(_V1)

    assert CID(cid) is cid
    assert CID.decode(cid) is cid


@pytest.mark.parametrize(
    'value',
    [
        '',
        'not-a-cid',
        'b',
        libipld.encode_multibase('b', _V1_BYTES + b'\x00'),
        libipld.encode_multibase('z', _V0_BYTES),
        _V0[:-1],
        b'',
        _V1_BYTES + b'\x00',
        _V1_BYTES + _V1_BYTES,
        _V1_BYTES[:-1],
        b'\x00' + _V1_BYTES,
        _V0_BYTES + b'\x00',
        bytes.fromhex('01719280808080808080800220') + bytes(32),
        bytes.fromhex('01f1001220') + bytes(32),
    ],
)
def test_cid_invalid(value) -> None:
    with pytest.raises(ValueError) as exc_info:
        CID(value)

    assert str(exc_info.value).startswith('Failed to decode CID. Invalid CID')


def test_cid_eq_and_hash() -> None:
    a, b, other = CID(_V1), CID(_V1_BYTES), CID(_V0)

    assert a == b
    assert (a != b) is False
    assert a != other
    assert hash(a) == hash(b)
    assert len({a, b, other}) == 2
    assert {a: 1}[b] == 1


@pytest.mark.parametrize('value', [_V1, _V1_BYTES, bytearray(_V1_BYTES), None, 1])
def test_cid_not_equal_to_other_types(value) -> None:
    cid = CID(_V1)

    assert cid != value
    assert value != cid
    assert (cid == value) is False
    assert value not in [cid]
    assert cid not in [value]


@pytest.mark.parametrize('value', [_V1, _V1_BYTES])
def test_cid_is_not_a_key_for_its_str_or_bytes(value) -> None:
    cid = CID(_V1)

    assert value not in {cid: 1}
    assert cid not in {value: 1}


def test_cid_has_no_order() -> None:
    with pytest.raises(TypeError):
        _ = CID(_V1) < CID(_V0)


def test_cid_is_immutable_and_final() -> None:
    cid = CID(_V1)

    with pytest.raises(AttributeError):
        cid.version = 0
    with pytest.raises(TypeError):

        class Sub(CID):
            pass


def test_cid_repr() -> None:
    assert repr(CID(_V1)) == f"CID('{_V1}')"
    assert repr(CID(_V0)) == f"CID('{_V0}')"


@pytest.mark.parametrize('protocol', range(pickle.HIGHEST_PROTOCOL + 1))
def test_cid_pickle(protocol) -> None:
    cid = CID(_V1)
    loaded = pickle.loads(pickle.dumps(cid, protocol=protocol))

    assert loaded == cid
    assert type(loaded) is CID


def test_cid_copy() -> None:
    cid = CID(_V1)

    assert copy.copy(cid) is cid
    assert copy.deepcopy({'a': cid})['a'] is cid


def test_cid_str_is_cached() -> None:
    cid = CID(_V1_BYTES)
    first = str(cid)

    assert str(cid) is first
    assert cid.encode() is first


def test_cid_str_keeps_canonical_input() -> None:
    class Str(str):
        pass

    text = ''.join(_V1)
    assert str(CID(text)) is text
    assert type(str(CID(Str(_V1)))) is str
    assert str(CID(_V1.upper())) == _V1


def test_multihash() -> None:
    mh = CID(_V1).hash

    assert mh == CID(_V1_BYTES).hash
    assert mh != CID(_V0).hash
    assert hash(mh) == hash(CID(_V1_BYTES).hash)
    assert repr(mh) == f'Multihash(code=18, size=32, digest={_V1_BYTES[4:].hex()})'
    with pytest.raises(AttributeError):
        mh.code = 0
    assert copy.copy(mh) is mh
    assert copy.deepcopy([mh])[0] is mh


@pytest.mark.parametrize('protocol', range(pickle.HIGHEST_PROTOCOL + 1))
def test_multihash_is_not_picklable(protocol) -> None:
    with pytest.raises(TypeError):
        pickle.dumps(CID(_V1).hash, protocol=protocol)


@pytest.mark.skipif(platform.python_implementation() != 'PyPy', reason='CPython refuses object.__new__(CID)')
def test_uninitialized_cid_raises() -> None:
    cid = object.__new__(CID)

    for access in (str, repr, bytes, CID.encode, libipld.encode_cid, libipld.encode_dag_cbor, pickle.dumps):
        with pytest.raises(ValueError):
            access(cid)
    for name in ('version', 'codec', 'hash'):
        with pytest.raises(ValueError):
            getattr(cid, name)

    mh = object.__new__(Multihash)
    assert (mh.code, mh.size, mh.digest) == (0, 0, b'')


@pytest.mark.skipif(platform.python_implementation() == 'PyPy', reason='PyPy allows object.__new__(CID)')
@pytest.mark.parametrize('cls', [CID, Multihash])
def test_object_new_is_refused(cls) -> None:
    with pytest.raises(TypeError):
        object.__new__(cls)


def test_decode_cid_and_encode_cid_accept_cid() -> None:
    cid = CID(_V1)

    assert libipld.decode_cid(cid) == libipld.decode_cid(_V1)
    assert libipld.encode_cid(cid) == _V1 == str(cid)
    assert libipld.encode_cid(CID(_V0)) == _V0


def _run_threads(target, count: int = 8) -> None:
    barrier = threading.Barrier(count)

    def run() -> None:
        barrier.wait()
        target()

    threads = [threading.Thread(target=run) for _ in range(count)]
    for thread in threads:
        thread.start()
    for thread in threads:
        thread.join()


def test_cid_str_from_many_threads() -> None:
    cid = CID(_V1_BYTES)
    results = []

    _run_threads(lambda: results.append(str(cid)))

    assert results == [_V1] * 8
    assert len({id(s) for s in results}) == 1


def test_cid_shared_between_threads() -> None:
    cids = [CID(_V1_BYTES[:-1] + bytes([i])) for i in range(256)]
    errors = []

    def work() -> None:
        try:
            for _ in range(20):
                seen = {cid: str(cid) for cid in cids}
                assert all(CID(s) == cid for cid, s in seen.items())
                assert libipld.decode_dag_cbor(libipld.encode_dag_cbor(cids)) == cids
        except (AssertionError, ValueError) as e:
            errors.append(e)

    _run_threads(work)

    assert errors == []
