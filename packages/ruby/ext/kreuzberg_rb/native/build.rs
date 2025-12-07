#[cfg(target_os = "macos")]
fn main() {
    println!("cargo:rustc-link-arg=-Wl,-undefined,dynamic_lookup");
    println!("cargo:rustc-link-arg=-Wl,-rpath,@loader_path");
    println!("cargo:rustc-link-arg=-Wl,-rpath,@loader_path/.");
}

#[cfg(target_os = "linux")]
fn main() {
    println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN");
    println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN/.");
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn main() {}
