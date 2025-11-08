use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let target = env::var("TARGET").unwrap();

    if target.contains("darwin") {
        println!("cargo:rustc-link-arg=-Wl,-rpath,@loader_path");
        println!("cargo:rustc-link-arg=-Wl,-rpath,@loader_path/.");
    } else if target.contains("linux") {
        println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN");
        println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN/.");
    }

    let target_dir = PathBuf::from(env::var("OUT_DIR").unwrap())
        .ancestors()
        .nth(3)
        .unwrap()
        .to_path_buf();

    let profile = env::var("PROFILE").unwrap();
    let bin_dir = target_dir.join(&profile);

    let lib_name = if target.contains("darwin") {
        "libpdfium.dylib"
    } else if target.contains("windows") {
        "pdfium.dll"
    } else {
        "libpdfium.so"
    };

    if let Ok(entries) = fs::read_dir(target_dir.join("build")) {
        for entry in entries.flatten() {
            if entry.file_name().to_string_lossy().starts_with("kreuzberg-") {
                let pdfium_lib = entry.path().join("out/pdfium/lib").join(lib_name);
                if pdfium_lib.exists() {
                    let dest = bin_dir.join(lib_name);
                    if let Err(e) = fs::copy(&pdfium_lib, &dest) {
                        eprintln!(
                            "Warning: Failed to copy {} to {}: {}",
                            pdfium_lib.display(),
                            dest.display(),
                            e
                        );
                    } else {
                        println!("cargo:warning=Copied {} to {}", pdfium_lib.display(), dest.display());

                        if target.contains("darwin") {
                            let binary_path = bin_dir.join("benchmark-harness");
                            let marker = bin_dir.join(".fix_pdfium_path");
                            fs::write(&marker, binary_path.to_string_lossy().as_bytes()).ok();
                        }
                    }
                    break;
                }
            }
        }
    }

    println!("cargo:rerun-if-changed=build.rs");
}
