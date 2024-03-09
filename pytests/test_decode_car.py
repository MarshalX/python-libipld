import os

import libipld
import pytest

from conftest import load_car_fixture


_DID = os.environ.get('CAR_REPO_DID', 'did:plc:w4es6sfh43zlht3bgrzi5qzq')  # default is public bot in bsky.app
_REPO_CAR_PATH = os.path.join(os.path.dirname(__file__), '..', 'data', 'repo.car')


@pytest.fixture(scope='session')
def car() -> bytes:
    return load_car_fixture(_DID, _REPO_CAR_PATH)


def test_decode_car(benchmark, car) -> None:
    header, blocks = benchmark(libipld.decode_car, car)

    assert 1 == header['version']
    assert isinstance(header['roots'], list)
    assert 1 == len(header['roots'])

    assert isinstance(blocks, dict)
    assert all(isinstance(k, str) for k in blocks.keys())
    assert all(len(k) == 59 for k in blocks.keys())
    assert all(isinstance(v, dict) for v in blocks.values())
    assert all(v for v in blocks.values())  # not empty dict


def test_decode_car_tuple(benchmark, car) -> None:
    header, blocks = benchmark(libipld.decode_car_tuple, car)

    assert 1 == header['version']
    assert isinstance(header['roots'], list)
    assert 1 == len(header['roots'])

    assert isinstance(blocks, tuple)
