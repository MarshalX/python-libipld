use std::fs;

use pyo3::prelude::*;

pub fn exec(iterations: u64) {
    let bench_file_name = "benchmarks/decode.dagcbor";

    let dag_cbor_bytes = fs::read(bench_file_name)
        .unwrap_or_else(|_| panic!("Could not open bench file '{}'", bench_file_name));

    Python::initialize();

    for _ in 0..iterations {
        Python::attach(|gil| {
            println!("{}", libipld::decode_dag_cbor(gil, &dag_cbor_bytes).is_ok());
        });
    }
}
