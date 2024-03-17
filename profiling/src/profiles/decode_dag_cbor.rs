use std::fs;

use pyo3::prelude::*;

pub fn exec(iterations: u64) {
    let bench_file_name = "benchmarks/decode.dagcbor";

    let dag_cbor_bytes = fs::read(bench_file_name)
        .expect(&format!("Could not open bench file '{}'", bench_file_name));

    pyo3::prepare_freethreaded_python();

    for _ in 0..iterations {
        Python::with_gil(|gil| {
            println!("{}", libipld::decode_dag_cbor(gil, &dag_cbor_bytes).is_ok());
        });
    }
}
