use std::fs;

use pyo3::prelude::*;
use pyo3::types::PyString;

pub fn exec(iterations: u64) {
    let bench_file_name = "benchmarks/encode.json";

    let json_data = fs::read_to_string(bench_file_name)
        .expect(&format!("Could not open bench file '{}'", bench_file_name));
    let json_str = json_data.as_str();

    pyo3::prepare_freethreaded_python();

    for _ in 0..iterations {
        Python::with_gil(|gil| {
            println!("{}", libipld::encode_dag_cbor(gil, &PyString::new(gil, json_str)).is_ok());
        });
    }
}
