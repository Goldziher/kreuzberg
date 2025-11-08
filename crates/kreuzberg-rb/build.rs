#[cfg(target_os = "macos")]
fn main() {
    println!("cargo:rustc-link-arg=-Wl,-undefined,dynamic_lookup");
}

#[cfg(not(target_os = "macos"))]
fn main() {}
