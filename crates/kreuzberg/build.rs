use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let target = env::var("TARGET").unwrap();
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    let (download_url, lib_name) = get_pdfium_url_and_lib(&target);

    let pdfium_dir = out_dir.join("pdfium");
    let lib_dir = pdfium_dir.join("lib");

    let lib_ext = if target.contains("darwin") {
        "dylib"
    } else if target.contains("windows") {
        "dll"
    } else {
        "so"
    };
    let lib_file = lib_dir.join(format!("libpdfium.{}", lib_ext));

    if !lib_file.exists() {
        eprintln!("Pdfium library not found, downloading for target: {}", target);
        eprintln!("Download URL: {}", download_url);
        download_and_extract_pdfium(&download_url, &pdfium_dir);
    } else {
        eprintln!("Pdfium library already present at {}", lib_file.display());
    }

    // On Windows, ensure pdfium.dll.lib is renamed to pdfium.lib
    // This handles both fresh downloads and cached versions
    if target.contains("windows") {
        let lib_dir = pdfium_dir.join("lib");
        let dll_lib = lib_dir.join("pdfium.dll.lib");
        let expected_lib = lib_dir.join("pdfium.lib");

        if dll_lib.exists() && !expected_lib.exists() {
            eprintln!("Renaming cached {} to {}", dll_lib.display(), expected_lib.display());
            fs::rename(&dll_lib, &expected_lib).expect("Failed to rename pdfium.dll.lib to pdfium.lib");
        }
    }

    let lib_dir = pdfium_dir.join("lib");
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-lib=dylib={}", lib_name);

    if target.contains("darwin") {
        println!("cargo:rustc-link-arg=-Wl,-rpath,@loader_path");
        println!("cargo:rustc-link-arg=-Wl,-rpath,@loader_path/.");
    } else if target.contains("linux") {
        println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN");
        println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN/.");
    }

    copy_lib_to_package(&pdfium_dir, &target);

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

    println!("cargo:rerun-if-changed=build.rs");
}

fn get_latest_version(repo: &str) -> String {
    use std::process::Command;

    let api_url = format!("https://api.github.com/repos/{}/releases/latest", repo);

    let output = Command::new("curl").args(["-s", &api_url]).output();

    if let Ok(output) = output
        && output.status.success()
    {
        let json = String::from_utf8_lossy(&output.stdout);
        if let Some(start) = json.find("\"tag_name\":") {
            // Find the opening quote after "tag_name":
            let after_colon = &json[start + "\"tag_name\":".len()..];
            if let Some(opening_quote) = after_colon.find('"')
                && let Some(closing_quote) = after_colon[opening_quote + 1..].find('"')
            {
                let tag_start = opening_quote + 1;
                let tag = &after_colon[tag_start..tag_start + closing_quote];
                return tag.split('/').next_back().unwrap_or(tag).to_string();
            }
        }
    }

    // Fallback to recent versions (October 2025)
    if repo.contains("bblanchon") {
        "7455".to_string()
    } else {
        "7469".to_string()
    }
}

fn get_pdfium_url_and_lib(target: &str) -> (String, String) {
    if target.contains("wasm") {
        // Check environment variable first, then try GitHub API, then fallback
        let version = env::var("PDFIUM_WASM_VERSION")
            .ok()
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| get_latest_version("paulocoutinhox/pdfium-lib"));
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
        let arch = if target.contains("aarch64") { "arm64" } else { "x64" };
        ("mac", arch)
    } else if target.contains("linux") {
        let arch = if target.contains("aarch64") {
            "arm64"
        } else if target.contains("arm") {
            "arm"
        } else {
            "x64"
        };
        ("linux", arch)
    } else if target.contains("windows") {
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

    // Check environment variable first, then try GitHub API, then fallback
    let version = env::var("PDFIUM_VERSION")
        .ok()
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| get_latest_version("bblanchon/pdfium-binaries"));
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

    eprintln!("Downloading Pdfium archive from: {}", url);
    let status = Command::new("curl")
        .args(["-f", "-L", "-o", archive_path.to_str().unwrap(), url])
        .status()
        .expect("Failed to execute curl");

    if !status.success() {
        panic!(
            "Failed to download Pdfium from {}. Check if the URL is valid and the version exists.",
            url
        );
    }

    // Verify the downloaded file is a gzip archive
    let file_type = Command::new("file")
        .arg(archive_path.to_str().unwrap())
        .output()
        .expect("Failed to check file type");

    let file_type_output = String::from_utf8_lossy(&file_type.stdout);
    eprintln!("Downloaded file type: {}", file_type_output.trim());

    if !file_type_output.to_lowercase().contains("gzip") && !file_type_output.to_lowercase().contains("compressed") {
        fs::remove_file(&archive_path).ok();
        panic!(
            "Downloaded file is not a valid gzip archive. URL may be incorrect or version unavailable: {}",
            url
        );
    }

    eprintln!("Extracting Pdfium archive...");
    let status = Command::new("tar")
        .args(["-xzf", archive_path.to_str().unwrap(), "-C", dest_dir.to_str().unwrap()])
        .status()
        .expect("Failed to execute tar");

    if !status.success() {
        fs::remove_file(&archive_path).ok();
        panic!("Failed to extract Pdfium archive from {}", url);
    }

    fs::remove_file(&archive_path).ok();

    // On Windows, bblanchon/pdfium-binaries names the import library pdfium.dll.lib
    // but the linker expects pdfium.lib. Rename it after extraction.
    let target = env::var("TARGET").unwrap();
    if target.contains("windows") {
        let lib_dir = dest_dir.join("lib");
        let dll_lib = lib_dir.join("pdfium.dll.lib");
        let expected_lib = lib_dir.join("pdfium.lib");

        if dll_lib.exists() {
            eprintln!("Renaming {} to {}", dll_lib.display(), expected_lib.display());
            fs::rename(&dll_lib, &expected_lib).expect("Failed to rename pdfium.dll.lib to pdfium.lib");
        } else {
            eprintln!("Warning: Expected {} not found after extraction", dll_lib.display());
        }
    }

    eprintln!("Pdfium downloaded and extracted successfully");
}

fn copy_lib_to_package(pdfium_dir: &Path, target: &str) {
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

    let crate_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let workspace_root = crate_dir.parent().unwrap().parent().unwrap();

    let python_dest_dir = workspace_root.join("packages").join("python").join("kreuzberg");
    if python_dest_dir.exists() {
        copy_lib_if_needed(&src_lib, &python_dest_dir.join(&lib_filename), "Python package");
    } else {
        eprintln!("Python package directory not found, skipping Python library copy");
    }

    let node_dest_dir = workspace_root.join("crates").join("kreuzberg-node");
    if node_dest_dir.exists() {
        copy_lib_if_needed(&src_lib, &node_dest_dir.join(&lib_filename), "Node.js package");
    } else {
        eprintln!("Node.js package directory not found, skipping Node library copy");
    }
}

fn copy_lib_if_needed(src: &Path, dest: &Path, package_name: &str) {
    use std::fs;

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
