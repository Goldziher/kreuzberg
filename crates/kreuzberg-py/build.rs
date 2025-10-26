use std::env;

fn main() {
    let target = env::var("TARGET").unwrap();

    if target.contains("darwin") {
        println!("cargo:rustc-link-arg=-Wl,-rpath,@loader_path");
    } else if target.contains("linux") {
        println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN");
    }

    println!("cargo:rerun-if-changed=build.rs");
}
