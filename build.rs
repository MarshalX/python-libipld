fn main() {
    pyo3_build_config::use_pyo3_cfgs();
    if matches!(
        pyo3_build_config::get().implementation,
        pyo3_build_config::PythonImplementation::CPython
    ) {
        println!("cargo:rustc-cfg=CPython");
    }
    println!("cargo:rustc-check-cfg=cfg(CPython)");
}
