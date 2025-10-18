fn main() {
    napi_build::setup();

    // Set rpath so the library can be found relative to the binary/library
    // This allows the Node.js addon to find libpdfium.dylib in the same directory
    let target = std::env::var("TARGET").unwrap();
    if target.contains("darwin") {
        // macOS: Use @loader_path to search relative to the loading binary
        println!("cargo:rustc-link-arg=-Wl,-rpath,@loader_path");
        println!("cargo:rustc-link-arg=-Wl,-rpath,@loader_path/.");
    } else if target.contains("linux") {
        // Linux: Use $ORIGIN to search relative to the loading binary
        println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN");
        println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN/.");
    }
}
