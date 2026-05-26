import pytest

from conftest import ENCODERS, VERSIONS


@pytest.mark.parametrize('lib', list(ENCODERS))
def test_encode(benchmark, lib, fixture_name, fixture_obj):
    benchmark.group = f'encode-{fixture_name}'
    benchmark.extra_info['op'] = 'encode'
    benchmark.extra_info['lib'] = lib
    benchmark.extra_info['version'] = VERSIONS[lib]
    benchmark.extra_info['fixture'] = fixture_name
    benchmark(ENCODERS[lib], fixture_obj)
