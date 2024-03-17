import libipld


def test_cid_decode_multibase() -> None:
    cid = libipld.decode_cid('bafyreig7jbijxpn4lfhvnvyuwf5u5jyhd7begxwyiqe7ingwxycjdqjjoa')
    assert 1 == cid['version']
    assert 113 == cid['codec']
    assert 18 == cid['hash']['code']
    assert 32 == cid['hash']['size']
    assert cid['hash']['size'] == len(cid['hash']['digest'])


def test_cid_decode_raw() -> None:
    cid = libipld.decode_cid(b'\x01q\x12 \xb6\x81\x1a\x1d\x7f\x8c\x17\x91\xdam\x1bO\x13m\xc0\xe2&y\xea\xfe\xaaX\xd6M~/\xaa\xd5\x89\x0e\x9d\x9c')
    assert 1 == cid['version']
    assert 113 == cid['codec']
    assert 18 == cid['hash']['code']
    assert 32 == cid['hash']['size']
    assert cid['hash']['size'] == len(cid['hash']['digest'])


def test_cid_encode_multibase() -> None:
    cid = 'bafyreig7jbijxpn4lfhvnvyuwf5u5jyhd7begxwyiqe7ingwxycjdqjjoa'
    assert cid == libipld.encode_cid(cid)  # because it's already encoded


def test_cid_encode_raw() -> None:
    raw_cid = b'\x01q\x12 \xb6\x81\x1a\x1d\x7f\x8c\x17\x91\xdam\x1bO\x13m\xc0\xe2&y\xea\xfe\xaaX\xd6M~/\xaa\xd5\x89\x0e\x9d\x9c'
    expected_cid_multibase = 'bafyreifwqenb274mc6i5u3i3j4jw3qhcez46v7vkldle27rpvlkysdu5tq'

    cid = libipld.decode_cid(raw_cid)
    cid_multibase = libipld.encode_cid(raw_cid)

    assert expected_cid_multibase == cid_multibase

    cid2 = libipld.decode_cid(expected_cid_multibase)

    assert cid == cid2

    # manual encoding for CID v1:
    assert expected_cid_multibase == libipld.encode_multibase('b', raw_cid)
