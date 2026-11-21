import libipld
import pytest


@pytest.mark.parametrize(
    'func,args',
    [
        (libipld.decode_car, ('str',)),
        (libipld.decode_dag_cbor, ('str',)),
        (libipld.decode_dag_cbor_multi, (123,)),
        (libipld.decode_multibase, (b'bytes',)),
        (libipld.encode_multibase, (1, b'data')),
    ],
)
def test_wrong_argument_type_raises_type_error(func, args) -> None:
    with pytest.raises(TypeError) as exc_info:
        func(*args)

    assert 'is not an instance of' in str(exc_info.value)


@pytest.mark.parametrize(
    'func,args',
    [
        (libipld.decode_cid, (123,)),
        (libipld.encode_cid, (123,)),
        (libipld.encode_multibase, ('u', 123)),
    ],
)
def test_wrong_str_or_bytes_argument_type_raises_value_error(func, args) -> None:
    with pytest.raises(ValueError) as exc_info:
        func(*args)

    assert 'Unsupported data type' in str(exc_info.value)


@pytest.mark.parametrize(
    'func,kwargs',
    [
        (libipld.decode_dag_cbor, {'canonical': 1}),
        (libipld.decode_dag_cbor_multi, {'canonical': 'yes'}),
    ],
)
def test_wrong_keyword_argument_type_raises_type_error(func, kwargs) -> None:
    with pytest.raises(TypeError) as exc_info:
        func(b'\x01', **kwargs)

    assert 'is not an instance of' in str(exc_info.value)


@pytest.mark.parametrize('func', [libipld.decode_dag_cbor, libipld.decode_dag_cbor_multi])
def test_canonical_is_keyword_only(func) -> None:
    with pytest.raises(TypeError):
        func(b'\x01', True)


@pytest.mark.parametrize('value', [1, None, [], memoryview(b'\x01\x71\x00\x00')])
def test_cid_wrong_argument_type_raises_type_error(value) -> None:
    with pytest.raises(TypeError) as exc_info:
        libipld.CID(value)

    assert 'CID() argument must be str, bytes, bytearray or CID' in str(exc_info.value)
