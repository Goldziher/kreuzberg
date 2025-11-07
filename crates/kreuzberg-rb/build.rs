#[cfg(target_os = "macos")]
fn main() {
    // Allow unresolved Ruby symbols at link time. They are resolved by the Ruby VM when
    // the extension is loaded, matching the behaviour of `extconf.rb`.
    println!("cargo:rustc-link-arg=-Wl,-undefined,dynamic_lookup");
}

#[cfg(not(target_os = "macos"))]
fn main() {}
