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
