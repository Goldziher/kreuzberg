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

fn get_latest_version(repo: &str) -> String {
    use std::process::Command;

    let api_url = format!("https://api.github.com/repos/{}/releases/latest", repo);

    // Try to fetch latest version via curl + jq
    let output = Command::new("curl")
        .args(["-s", &api_url])
        .output();

    if let Ok(output) = output {
        if output.status.success() {
            let json = String::from_utf8_lossy(&output.stdout);
            // Simple JSON parsing - extract tag_name
            if let Some(start) = json.find("\"tag_name\":") {
                if let Some(quote_start) = json[start..].find('"').and_then(|i| json[start + i + 1..].find('"')) {
                    let tag_start = start + json[start..].find('"').unwrap() + 1;
                    let tag = &json[tag_start..tag_start + quote_start];
                    // Extract version from tag (e.g., "chromium/7469" -> "7469" or "7442b" -> "7442b")
                    return tag.split('/').last().unwrap_or(tag).to_string();
                }
            }
        }
    }

    // Fallback versions if API fetch fails
    if repo.contains("bblanchon") {
        "7469".to_string()
    } else {
        "7442b".to_string()
    }
}

fn get_pdfium_url_and_lib(target: &str) -> (String, String) {
    // Determine platform and architecture
    if target.contains("wasm") {
        // WASM build - use paulocoutinhox/pdfium-lib
        let version = get_latest_version("paulocoutinhox/pdfium-lib");
        eprintln!("Using pdfium-lib version: {}", version);

        let wasm_arch = if target.contains("wasm32") { "wasm32" } else { "wasm64" };
        return (
            format!(
                "https://github.com/paulocoutinhox/pdfium-lib/releases/download/{}/pdfium-{}.tar.gz",
                version, wasm_arch
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
    let version = get_latest_version("bblanchon/pdfium-binaries");
    eprintln!("Using pdfium-binaries version: {}", version);

    let url = format!(
        "https://github.com/bblanchon/pdfium-binaries/releases/download/chromium/{}/pdfium-{}-{}.tgz",
        version, platform, arch
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

    if !src_lib.exists() {
        eprintln!("Source library not found: {}", src_lib.display());
        return;
    }

    // Path: crates/kreuzberg -> crates -> workspace_root
    let crate_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let workspace_root = crate_dir
        .parent()  // crates/kreuzberg -> crates
        .unwrap()
        .parent()  // crates -> workspace_root
        .unwrap();

    // Copy to Python package directory
    let python_dest_dir = workspace_root.join("packages").join("python").join("kreuzberg");
    if python_dest_dir.exists() {
        copy_lib_if_needed(&src_lib, &python_dest_dir.join(&lib_filename), "Python package");
    } else {
        eprintln!("Python package directory not found, skipping Python library copy");
    }

    // Copy to Node.js package directory (kreuzberg-node crate)
    let node_dest_dir = workspace_root.join("crates").join("kreuzberg-node");
    if node_dest_dir.exists() {
        copy_lib_if_needed(&src_lib, &node_dest_dir.join(&lib_filename), "Node.js package");
    } else {
        eprintln!("Node.js package directory not found, skipping Node library copy");
    }
}

fn copy_lib_if_needed(src: &Path, dest: &Path, package_name: &str) {
    use std::fs;

    // Only copy if source is newer or dest doesn't exist
    let should_copy = if dest.exists() {
        let src_metadata = fs::metadata(src).ok();
        let dest_metadata = fs::metadata(dest).ok();
        match (src_metadata, dest_metadata) {
            (Some(src), Some(dest)) => src.modified().ok() > dest.modified().ok(),
            _ => true,
        }
    } else {
        true
    };

    if should_copy {
        match fs::copy(src, dest) {
            Ok(_) => eprintln!("Copied {} to {} ({})", src.display(), dest.display(), package_name),
            Err(e) => eprintln!("Failed to copy library to {}: {}", package_name, e),
        }
    }
}
