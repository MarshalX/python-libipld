## Contibuting to python-libipld

This project is a small, single-file wrapper around Rust crates like `cid`, `cbor4ii`, and `multibase`, exposing a Python API through `PyO3`. Despite its size, performance matters a lot.

The project uses `uv` package manager. Installing UV: https://docs.astral.sh/uv/getting-started/installation/

Commands for quick start:
```shell
# install deps
uv sync --group all

# compile and install using maturin directly (faster and better for developing)
uv run maturin develop

# compile and install using pip and maturin backend
uv pip install -v -e .

# run all tests
uv run pytest

# run the most important benchmarks
uv run pytest . -m benchmark_main --benchmark-enable

# run lint and fmt
cargo clippy && cargo fmt
```

### Performance

Two key points:

1. Python-side benchmarks

    We use `pytest-benchmark` and run all benchmarks from the Python side. `CodSpeed` is used in CI/CD, but it relies on CPU simulation. The best comparison is always on your local machine.

    First, capture the baseline from the `main` branch. This records performance relative to your hardware:
    ```shell
    # clone and checkout main branch
    uv pip install -v -e .  
    # run the most important benchmarks
    uv run pytest . -m benchmark_main --benchmark-enable --benchmark-save=main
    ```

    Then, on your feature branch, run the same benchmarks but save under a different name (`--benchmark-save` argument)
    ```shell
    # checkout your branch
    uv pip install -v -e .
    uv run pytest . -m benchmark_main --benchmark-enable --benchmark-save=your_feature
    ```

    Finally, compare results:
    ```shell
    uv run pytest-benchmark compare --group-by="name" 
    ```

    Notes:
    - Benchmark data is stored under `.benchmarks`.
    - You can delete old snapshots during local development.

2. Rust-side benchmarks

    We also maintain Rust benchmarks, but they mainly exist for profiling and diagnosing performance issues. They work better with tools like flamegraph than when forced into a Python boundary. See the project's [Makefile](Makefile) for details.

### Testing

All tests target the Python-facing API, which is why the `pytest` directory exists.

Any segfaults or Rust panics **must** be handled safely and must never crash the Python interpreter. Every error must be catchable at the Python layer.

### Style

Use `cargo fmt` and `cargo clippy`. CI will block your PR if formatting or linting fails.

### Things to care about

This library is used in:
- DAG-CBOR benchmarks for Python: https://github.com/DavidBuchanan314/dag-cbor-benchmark
- DASL Testing: https://hyphacoop.github.io/dasl-testing/

Keep these in mind and consider running their test suites against your feature branch locally.
