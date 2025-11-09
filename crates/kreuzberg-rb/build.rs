#[cfg(target_os = "macos")]
fn main() {
    println!("cargo:rustc-link-arg=-Wl,-undefined,dynamic_lookup");
    // Set rpath to look for libpdfium.dylib in the same directory as the Ruby extension
    println!("cargo:rustc-link-arg=-Wl,-rpath,@loader_path");
    println!("cargo:rustc-link-arg=-Wl,-rpath,@loader_path/.");
}

#[cfg(target_os = "linux")]
fn main() {
    // Set rpath to look for libpdfium.so in the same directory as the Ruby extension
    println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN");
    println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN/.");
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn main() {}
