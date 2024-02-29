import libipld
import pytest


def test_multibase_encode() -> None:
    assert libipld.encode_multibase('7', 'yes mani !') == '7362625631006654133464440102'
    assert libipld.encode_multibase('u', b'yes mani !') == 'ueWVzIG1hbmkgIQ'
    assert libipld.encode_multibase(
        'z',
        b'\xe7\x01\x03\xe2@y~I\xd8W\xdb}\xfb\xb1\xc4uG\xd6ec\xf8]\xb3\x16\xd0;\x11S\x19\xcfX\xf8\xb5QB'
    ) == 'zQ3shusJHhGZ21fxVrCSs4TNNYQp84yDcT7XhpR2thAvV26wB'
    assert libipld.encode_multibase(
        'z',
        bytearray(b'\xe7\x01\x03\xe2@y~I\xd8W\xdb}\xfb\xb1\xc4uG\xd6ec\xf8]\xb3\x16\xd0;\x11S\x19\xcfX\xf8\xb5QB')
    ) == 'zQ3shusJHhGZ21fxVrCSs4TNNYQp84yDcT7XhpR2thAvV26wB'


def test_multibase_decode() -> None:
    code, b = libipld.decode_multibase('zQ3shusJHhGZ21fxVrCSs4TNNYQp84yDcT7XhpR2thAvV26wB')
    assert code == 'z'
    assert b == b'\xe7\x01\x03\xe2@y~I\xd8W\xdb}\xfb\xb1\xc4uG\xd6ec\xf8]\xb3\x16\xd0;\x11S\x19\xcfX\xf8\xb5QB'
    assert bytearray(b) == bytearray(
        b'\xe7\x01\x03\xe2@y~I\xd8W\xdb}\xfb\xb1\xc4uG\xd6ec\xf8]\xb3\x16\xd0;\x11S\x19\xcfX\xf8\xb5QB'
    )

    code, b = libipld.decode_multibase('ueWVzIG1hbmkgIQ')
    assert code == 'u'
    assert b == b'yes mani !'

    libipld.decode_multibase('BPFSXGIDNMFXGSIBB')
    libipld.decode_multibase('7362625631006654133464440102')


def test_multibase_encode_unsupported_type() -> None:
    with pytest.raises(ValueError) as exc_info:
        libipld.encode_multibase('u', 123)

    assert "Unsupported data type" in str(exc_info.value)


def test_multibase_decode_unknown_base_error() -> None:
    with pytest.raises(ValueError) as exc_info:
        libipld.decode_multibase('dddddd')

    assert 'Unknown base code' in str(exc_info.value)


def test_multibase_decode_invalid_base_string_error() -> None:
    with pytest.raises(ValueError) as exc_info:
        print(libipld.decode_multibase('u123'))

    assert 'Invalid base string' in str(exc_info.value)


def test_multibase_encode_kwargs() -> None:
    assert libipld.encode_multibase(code='7', data='yes mani !') == '7362625631006654133464440102'
