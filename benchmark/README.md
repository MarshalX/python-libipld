# benchmark

DAG-CBOR encode/decode benchmark across Python implementations.

Compared:
- [`libipld`](https://github.com/MarshalX/python-libipld) (Rust)
- [`cbrrr`](https://github.com/DavidBuchanan314/dag-cbrrr) (C)
- [`py-ipld-dag`](https://github.com/ipld/py-ipld-dag) (Python wrapper over [`cbor2`](https://github.com/agronholm/cbor2) with `canonical=True`; cbor2 itself is Rust)
- [`dag_cbor`](https://github.com/hashberg-io/dag-cbor) (pure Python, used as the 1× baseline)

Fixtures: `canada.json`, `citm_catalog.json`, `github.json`, `twitter.json` (loaded from `../data/`, parsed once, then encoded to DAG-CBOR via `libipld` for the decode benchmarks).

## Run

```sh
./run.sh           # encode + decode
./run.sh encode    # encode only
./run.sh decode    # decode only
```

Outputs `serialization.png` and `deserialization.png` next to `chart.py`. Raw history goes into `.benchmarks/` (pytest-benchmark autosave).

## Which `libipld` is measured?

By default, `requirements.txt` pulls the **published PGO-optimized wheel** from PyPI. This is what users actually get when they `pip install libipld`, so the charts reflect real-world performance.

To benchmark a locally-built PGO wheel instead, edit the `libipld` line in `requirements.txt`:

```
libipld @ file:///abs/path/to/libipld-*.whl
```

Building a local PGO wheel is out of scope here.

## Filter

Skip a slow library (e.g. pure-Python `dag_cbor` on `canada`):

```sh
uv run --with-requirements requirements.txt --with-editable .. \
    pytest --benchmark-enable --benchmark-json=results.json -k "not dag_cbor"
uv run --with-requirements requirements.txt python chart.py results.json
```
