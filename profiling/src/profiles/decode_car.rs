use std::fs;

use pyo3::prelude::*;

pub fn exec(iterations: u64) {
    let bench_file_name = "benchmarks/repo.car";

    let car_bytes = fs::read(bench_file_name)
        .unwrap_or_else(|_| panic!("Could not open bench file '{}'", bench_file_name));

    Python::initialize();

    Python::attach(|gil| {
        unsafe { pyo3::ffi::Py_SetRecursionLimit(10_000) };
        let _ = gil;
    });

    for _ in 0..iterations {
        Python::attach(|gil| {
            println!("{}", libipld::decode_car(gil, &car_bytes).is_ok());
        });
    }
}
