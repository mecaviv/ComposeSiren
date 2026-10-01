//! Generates the C header, `include/clic_composesiren.h`, from `src/ffi.rs`.
//!
//! The header is committed, so that C and C++ consumers find it without
//! building first. cbindgen only rewrites it when its content changes.

use std::env;
use std::path::PathBuf;

fn main() {
    let crate_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("cargo sets it"));
    println!("cargo::rerun-if-changed=src");
    println!("cargo::rerun-if-changed=cbindgen.toml");

    let config = cbindgen::Config::from_file(crate_dir.join("cbindgen.toml"))
        .expect("cbindgen.toml is valid");
    cbindgen::Builder::new()
        .with_crate(&crate_dir)
        .with_config(config)
        .generate()
        .expect("the C header can be generated from src/ffi.rs")
        .write_to_file(crate_dir.join("include/clic_composesiren.h"));
}
