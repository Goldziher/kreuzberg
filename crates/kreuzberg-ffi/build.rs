use std::env;

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");

    let config = cbindgen::Config::from_file("cbindgen.toml").expect("Failed to load cbindgen config");

    cbindgen::generate_with_config(&crate_dir, config)
        .expect("Failed to generate C bindings")
        .write_to_file("kreuzberg.h");

    // Tell cargo to rerun if the cbindgen config changes
    println!("cargo:rerun-if-changed=cbindgen.toml");
    println!("cargo:rerun-if-changed=src/lib.rs");
}
