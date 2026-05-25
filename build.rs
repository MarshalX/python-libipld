fn main() {
    if matches!(
        pyo3_build_config::get().implementation,
        pyo3_build_config::PythonImplementation::CPython
    ) {
        println!("cargo:rustc-cfg=CPython");
    }
    println!("cargo:rustc-check-cfg=cfg(CPython)");
}
