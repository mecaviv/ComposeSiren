//! Generates `include/composesiren_record.h` from `src/ffi.rs`.
//!
//! The header is committed so C++ can include it before the first Cargo build.
//! cbindgen rewrites it only when the content changes.

use std::env;
use std::path::PathBuf;

fn main() {
    let crate_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("cargo sets it"));
    println!("cargo::rerun-if-changed=src");
    println!("cargo::rerun-if-changed=cbindgen.toml");

    let config = cbindgen::Config::from_file(crate_dir.join("cbindgen.toml")).expect("cbindgen.toml is valid");
    cbindgen::Builder::new()
        .with_crate(&crate_dir)
        .with_config(config)
        .generate()
        .expect("the C header can be generated from src/ffi.rs")
        .write_to_file(crate_dir.join("include/composesiren_record.h"));
}
