use std::env;

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");

    let config = cbindgen::Config::from_file("cbindgen.toml").expect("Failed to load cbindgen config");

    cbindgen::generate_with_config(&crate_dir, config)
        .expect("Failed to generate C bindings")
        .write_to_file("kreuzberg.h");

    // Configure RPATH on macOS for libpdfium.dylib resolution
    // This ensures that @rpath/libpdfium.dylib can be found at runtime
    #[cfg(target_os = "macos")]
    {
        // Use @loader_path to make the library relocatable
        // @loader_path points to the directory containing the dylib being loaded
        println!("cargo:rustc-link-arg=-rpath");
        println!("cargo:rustc-link-arg=@loader_path");

        // Also add the target/release directory as a fallback
        // This handles the case where the library is loaded from the build output
        println!("cargo:rustc-link-arg=-rpath");
        println!("cargo:rustc-link-arg=@executable_path/../target/release");
    }

    // Tell cargo to rerun if the cbindgen config changes
    println!("cargo:rerun-if-changed=cbindgen.toml");
    println!("cargo:rerun-if-changed=src/lib.rs");
}
