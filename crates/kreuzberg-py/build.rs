use std::env;

fn main() {
    let target = env::var("TARGET").unwrap();

    // Set rpath so the library can be found relative to the Python extension
    // This allows the Python extension to find libpdfium.dylib in the same directory
    //
    // Note: The library install name is fixed post-build by Taskfile.yaml or
    // _setup_lib_path.py to use @loader_path instead of ./libpdfium.dylib
    if target.contains("darwin") {
        // macOS: Use @loader_path to search relative to the loading binary
        println!("cargo:rustc-link-arg=-Wl,-rpath,@loader_path");
    } else if target.contains("linux") {
        // Linux: Use $ORIGIN to search relative to the loading binary
        println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN");
    }

    // Rerun if build script changes
    println!("cargo:rerun-if-changed=build.rs");
}
