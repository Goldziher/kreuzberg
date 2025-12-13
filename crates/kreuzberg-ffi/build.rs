use std::env;

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");

    let config = cbindgen::Config::from_file("cbindgen.toml").expect("Failed to load cbindgen config");

    cbindgen::generate_with_config(&crate_dir, config)
        .expect("Failed to generate C bindings")
        .write_to_file("kreuzberg.h");

    // Generate pkg-config files
    let pc_template = std::fs::read_to_string("kreuzberg-ffi.pc.in").expect("Failed to read pkg-config template");

    let version = env::var("CARGO_PKG_VERSION").unwrap();
    let repo_root = std::path::Path::new(&crate_dir).parent().unwrap().parent().unwrap();
    let dev_prefix = repo_root.to_string_lossy();

    // Platform-specific private libs (use CARGO_CFG_TARGET_OS for cross-compilation support)
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_else(|_| "unknown".to_string());
    let libs_private = match target_os.as_str() {
        "linux" => "-lpthread -ldl -lm",
        "macos" => "-framework CoreFoundation -framework Security -lpthread",
        "windows" => "-lpthread -lws2_32 -luserenv -lbcrypt -static-libgcc -static-libstdc++",
        _ => "",
    };

    // Development version (for monorepo use) - use actual monorepo paths
    let dev_libdir = format!("{}/target/release", dev_prefix);
    let dev_includedir = format!("{}/crates/kreuzberg-ffi", dev_prefix);
    let dev_pc = format!(
        r#"prefix={}
exec_prefix=${{prefix}}
libdir={}
includedir={}

Name: kreuzberg-ffi
Description: C FFI bindings for Kreuzberg document intelligence library
Version: {}
URL: https://kreuzberg.dev
Libs: -L${{libdir}} -lkreuzberg_ffi
Libs.private: {}
Cflags: -I${{includedir}}
"#,
        dev_prefix, dev_libdir, dev_includedir, version, libs_private
    );
    std::fs::write("kreuzberg-ffi.pc", dev_pc).expect("Failed to write development pkg-config");

    // Installation version (for release artifacts)
    let install_pc = pc_template
        .replace("@PREFIX@", "/usr/local")
        .replace("@VERSION@", &version)
        .replace("@LIBS_PRIVATE@", libs_private);
    std::fs::write("kreuzberg-ffi-install.pc", install_pc).expect("Failed to write installation pkg-config");

    #[cfg(target_os = "macos")]
    {
        println!("cargo:rustc-link-arg=-rpath");
        println!("cargo:rustc-link-arg=@loader_path");

        println!("cargo:rustc-link-arg=-rpath");
        println!("cargo:rustc-link-arg=@executable_path/../target/release");
    }

    println!("cargo:rerun-if-changed=cbindgen.toml");
    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=kreuzberg-ffi.pc.in");
}
