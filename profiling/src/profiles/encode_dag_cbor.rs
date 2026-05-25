use std::fs;

use pyo3::prelude::*;
use pyo3::types::PyString;

pub fn exec(iterations: u64) {
    let bench_file_name = "benchmarks/encode.json";

    let json_data = fs::read_to_string(bench_file_name)
        .unwrap_or_else(|_| panic!("Could not open bench file '{}'", bench_file_name));
    let json_str = json_data.as_str();

    Python::initialize();

    for _ in 0..iterations {
        Python::attach(|gil| {
            println!(
                "{}",
                libipld::encode_dag_cbor(gil, &PyString::new(gil, json_str)).is_ok()
            );
        });
    }
}
