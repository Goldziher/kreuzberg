use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let target = env::var("TARGET").unwrap();
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    // Determine which pdfium binary to download based on target
    let (download_url, lib_name) = get_pdfium_url_and_lib(&target);

    // Download and extract pdfium binary
    let pdfium_dir = out_dir.join("pdfium");
    let lib_dir = pdfium_dir.join("lib");

    // Determine library file name
    let lib_ext = if target.contains("darwin") {
        "dylib"
    } else if target.contains("windows") {
        "dll"
    } else {
        "so"
    };
    let lib_file = lib_dir.join(format!("libpdfium.{}", lib_ext));

    // Only download if library doesn't exist
    if !lib_file.exists() {
        eprintln!("Pdfium library not found, downloading for target: {}", target);
        eprintln!("Download URL: {}", download_url);
        download_and_extract_pdfium(&download_url, &pdfium_dir);
    } else {
        eprintln!("Pdfium library already present at {}", lib_file.display());
    }

    // Set up linking
    let lib_dir = pdfium_dir.join("lib");
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-lib=dylib={}", lib_name);

    // Set rpath so the library can be found relative to the binary/library
    // This allows the Python extension to find libpdfium.dylib in the same directory
    if target.contains("darwin") {
        // macOS: Use @loader_path to search relative to the loading binary
        println!("cargo:rustc-link-arg=-Wl,-rpath,@loader_path");
        println!("cargo:rustc-link-arg=-Wl,-rpath,@loader_path/.");
    } else if target.contains("linux") {
        // Linux: Use $ORIGIN to search relative to the loading binary
        println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN");
        println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN/.");
    }

    // Copy library to Python package location for maturin to include in wheel
    copy_lib_to_package(&pdfium_dir, &target);

    // Platform-specific system libraries
    if target.contains("darwin") {
        println!("cargo:rustc-link-lib=framework=CoreFoundation");
        println!("cargo:rustc-link-lib=framework=CoreGraphics");
        println!("cargo:rustc-link-lib=framework=CoreText");
        println!("cargo:rustc-link-lib=framework=AppKit");
        println!("cargo:rustc-link-lib=dylib=c++");
    } else if target.contains("linux") {
        println!("cargo:rustc-link-lib=dylib=stdc++");
        println!("cargo:rustc-link-lib=dylib=m");
    } else if target.contains("windows") {
        println!("cargo:rustc-link-lib=dylib=gdi32");
        println!("cargo:rustc-link-lib=dylib=user32");
        println!("cargo:rustc-link-lib=dylib=advapi32");
    }

    // Rerun if build script changes
    println!("cargo:rerun-if-changed=build.rs");
}

fn get_pdfium_url_and_lib(target: &str) -> (String, String) {
    // Pdfium versions (latest from both repos)
    const PDFIUM_VERSION_BBLANCHON: &str = "7469";
    const PDFIUM_VERSION_PAULOCOUTINHOX: &str = "7442b";

    // Determine platform and architecture
    if target.contains("wasm") {
        // WASM build - use paulocoutinhox/pdfium-lib
        let wasm_arch = if target.contains("wasm32") { "wasm32" } else { "wasm64" };
        return (
            format!(
                "https://github.com/paulocoutinhox/pdfium-lib/releases/download/{}/pdfium-{}.tar.gz",
                PDFIUM_VERSION_PAULOCOUTINHOX, wasm_arch
            ),
            "pdfium".to_string(),
        );
    }

    let (platform, arch) = if target.contains("darwin") {
        // macOS
        let arch = if target.contains("aarch64") { "arm64" } else { "x64" };
        ("mac", arch)
    } else if target.contains("linux") {
        // Linux
        let arch = if target.contains("aarch64") {
            "arm64"
        } else if target.contains("arm") {
            "arm"
        } else {
            "x64"
        };
        ("linux", arch)
    } else if target.contains("windows") {
        // Windows
        let arch = if target.contains("aarch64") {
            "arm64"
        } else if target.contains("i686") {
            "x86"
        } else {
            "x64"
        };
        ("win", arch)
    } else {
        panic!("Unsupported target platform: {}", target);
    };

    // Use bblanchon/pdfium-binaries for native platforms
    let url = format!(
        "https://github.com/bblanchon/pdfium-binaries/releases/download/chromium%2F{}/pdfium-{}-{}.tgz",
        PDFIUM_VERSION_BBLANCHON, platform, arch
    );

    (url, "pdfium".to_string())
}

fn download_and_extract_pdfium(url: &str, dest_dir: &PathBuf) {
    use std::process::Command;

    fs::create_dir_all(dest_dir).expect("Failed to create pdfium directory");

    let archive_path = dest_dir.join("pdfium.tar.gz");

    // Download using curl (available on all platforms)
    eprintln!("Downloading Pdfium archive...");
    let status = Command::new("curl")
        .args(["-L", "-o", archive_path.to_str().unwrap(), url])
        .status()
        .expect("Failed to execute curl");

    if !status.success() {
        panic!("Failed to download Pdfium from {}", url);
    }

    // Extract using tar (available on all platforms)
    eprintln!("Extracting Pdfium archive...");
    let status = Command::new("tar")
        .args(["-xzf", archive_path.to_str().unwrap(), "-C", dest_dir.to_str().unwrap()])
        .status()
        .expect("Failed to execute tar");

    if !status.success() {
        panic!("Failed to extract Pdfium archive");
    }

    // Clean up archive
    fs::remove_file(&archive_path).ok();

    eprintln!("Pdfium downloaded and extracted successfully");
}

fn copy_lib_to_package(pdfium_dir: &Path, target: &str) {
    use std::fs;

    // Determine library file extension
    let lib_ext = if target.contains("darwin") {
        "dylib"
    } else if target.contains("windows") {
        "dll"
    } else {
        "so"
    };

    let lib_filename = format!("libpdfium.{}", lib_ext);
    let src_lib = pdfium_dir.join("lib").join(&lib_filename);

    // Copy to Python package directory
    // Path: crates/kreuzberg -> crates -> workspace_root -> packages/python/kreuzberg
    let crate_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let workspace_root = crate_dir
        .parent()  // crates/kreuzberg -> crates
        .unwrap()
        .parent()  // crates -> workspace_root
        .unwrap();

    let dest_dir = workspace_root.join("packages").join("python").join("kreuzberg");

    // Only copy if destination directory exists (we're in the monorepo)
    if !dest_dir.exists() {
        eprintln!("Python package directory not found, skipping library copy");
        return;
    }

    let dest_lib = dest_dir.join(&lib_filename);
    if src_lib.exists() {
        // Only copy if source is newer or dest doesn't exist
        let should_copy = if dest_lib.exists() {
            let src_metadata = fs::metadata(&src_lib).ok();
            let dest_metadata = fs::metadata(&dest_lib).ok();
            match (src_metadata, dest_metadata) {
                (Some(src), Some(dest)) => src.modified().ok() > dest.modified().ok(),
                _ => true,
            }
        } else {
            true
        };

        if should_copy {
            match fs::copy(&src_lib, &dest_lib) {
                Ok(_) => eprintln!("Copied {} to {}", src_lib.display(), dest_lib.display()),
                Err(e) => eprintln!("Failed to copy library: {}", e),
            }
        }
    } else {
        eprintln!("Source library not found: {}", src_lib.display());
    }
}
