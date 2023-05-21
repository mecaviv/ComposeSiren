#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(unused_imports)]
#![allow(non_upper_case_globals)]
#![allow(unused_variables)]
extern crate cbindgen;

use std::env;
use std::fs::File;
use std::path::Path;

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let result =
        cbindgen::Builder::new()
            .with_crate(crate_dir)
            .generate();
    match result {
        Ok(bindings) => {


            bindings.write_to_file("../../Source/generated/composesirenrs.h");
            println!("generated C binding.");
        }
        Err(error) => {
            println!("failed to do C binding generation: {}", error.to_string());
        }
    }
}