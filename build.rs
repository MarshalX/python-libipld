use std::{env, fs, path::Path};

fn main() {
    pyo3_build_config::use_pyo3_cfgs();
    let config = pyo3_build_config::get();
    if matches!(
        config.implementation(),
        pyo3_build_config::PythonImplementation::CPython
    ) {
        println!("cargo:rustc-cfg=CPython");
    }
    println!("cargo:rustc-check-cfg=cfg(CPython)");

    let link = match env::var("CARGO_CFG_TARGET_OS").as_deref() {
        Ok("windows") => format!(
            r#"#[link(name = "{}", kind = "raw-dylib")]"#,
            config.lib_name().unwrap()
        ),
        _ => String::new(),
    };
    let out = Path::new(&env::var("OUT_DIR").unwrap()).join("extern_libpython.rs");
    let src = format!(
        "macro_rules! extern_libpython {{ ($($t:tt)*) => {{ {link} extern \"C\" {{ $($t)* }} }} }}"
    );
    fs::write(out, src).unwrap();
}
