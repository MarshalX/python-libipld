use std::fs;

use pyo3::prelude::*;

pub fn exec(iterations: u64) {
    let bench_file_name = "benchmarks/repo.car";

    let car_bytes = fs::read(bench_file_name)
        .expect(&format!("Could not open bench file '{}'", bench_file_name));

    pyo3::prepare_freethreaded_python();

    for _ in 0..iterations {
        Python::with_gil(|gil| {
            println!("{}", libipld::decode_car(gil, &car_bytes).is_ok());
        });
    }
}
