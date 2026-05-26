import pytest

from conftest import DECODERS, VERSIONS


@pytest.mark.parametrize('lib', list(DECODERS))
def test_decode(benchmark, lib, fixture_name, fixture_bytes):
    benchmark.group = f'decode-{fixture_name}'
    benchmark.extra_info['op'] = 'decode'
    benchmark.extra_info['lib'] = lib
    benchmark.extra_info['version'] = VERSIONS[lib]
    benchmark.extra_info['fixture'] = fixture_name
    benchmark(DECODERS[lib], fixture_bytes)
