import json
from importlib.metadata import version
from pathlib import Path

import pytest

import libipld

try:
    import cbrrr
except ImportError:
    cbrrr = None

try:
    import dag_cbor
except ImportError:
    dag_cbor = None


FIXTURES = ['canada', 'citm_catalog', 'github', 'twitter']
DATA_DIR = Path(__file__).parent.parent / 'data'

DECODERS = {'libipld': libipld.decode_dag_cbor}
ENCODERS = {'libipld': libipld.encode_dag_cbor}
VERSIONS = {'libipld': version('libipld')}
if cbrrr is not None:
    DECODERS['cbrrr'] = cbrrr.decode_dag_cbor
    ENCODERS['cbrrr'] = cbrrr.encode_dag_cbor
    VERSIONS['cbrrr'] = version('cbrrr')
if dag_cbor is not None:
    DECODERS['dag_cbor'] = dag_cbor.decode
    ENCODERS['dag_cbor'] = dag_cbor.encode
    VERSIONS['dag_cbor'] = version('dag-cbor')


@pytest.fixture(scope='session', params=FIXTURES)
def fixture_name(request):
    return request.param


@pytest.fixture(scope='session')
def fixture_obj(fixture_name):
    with open(DATA_DIR / f'{fixture_name}.json') as f:
        return json.load(f)


@pytest.fixture(scope='session')
def fixture_bytes(fixture_obj):
    return libipld.encode_dag_cbor(fixture_obj)
