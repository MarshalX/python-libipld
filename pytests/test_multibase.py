import libipld
import pytest


def test_multibase_encode() -> None:
    libipld.encode_multibase('7', 'yes mani !')
    libipld.encode_multibase('u', b'yes mani !')
    libipld.encode_multibase('z', b'\xe7\x01\x03\xe2@y~I\xd8W\xdb}\xfb\xb1\xc4uG\xd6ec\xf8]\xb3\x16\xd0;\x11S\x19\xcfX\xf8\xb5QB')


def test_multibase_decode() -> None:
    libipld.decode_multibase('zQ3shusJHhGZ21fxVrCSs4TNNYQp84yDcT7XhpR2thAvV26wB')
    libipld.decode_multibase('BPFSXGIDNMFXGSIBB')
    libipld.decode_multibase('ueWVzIG1hbmkgIQ')
    libipld.decode_multibase('7362625631006654133464440102')


def test_multibase_unknown_base_error() -> None:
    with pytest.raises(ValueError) as exc_info:
        libipld.decode_multibase('dddddd')

        assert str(exc_info.value) == 'Unknown base: d'
