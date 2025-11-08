use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

fn main() {
    let target = env::var("TARGET").unwrap();
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    println!("cargo::rustc-check-cfg=cfg(coverage)");

    let (download_url, lib_name) = get_pdfium_url_and_lib(&target);

    let pdfium_dir = out_dir.join("pdfium");

    if let Some(prebuilt) = env::var_os("KREUZBERG_PDFIUM_PREBUILT") {
        let prebuilt_path = PathBuf::from(prebuilt);
        if prebuilt_path.exists() {
            prepare_prebuilt_pdfium(&prebuilt_path, &pdfium_dir)
                .unwrap_or_else(|err| panic!("Failed to copy Pdfium from {}: {}", prebuilt_path.display(), err));
        } else {
            panic!(
                "Environment variable KREUZBERG_PDFIUM_PREBUILT points to '{}' but the directory does not exist",
                prebuilt_path.display()
            );
        }
    }

    let (runtime_lib_name, runtime_subdir) = runtime_library_info(&target);
    let runtime_lib_path = pdfium_dir.join(runtime_subdir).join(&runtime_lib_name);
    let import_lib_exists = if target.contains("windows") {
        let lib_dir = pdfium_dir.join("lib");
        lib_dir.join("pdfium.lib").exists() || lib_dir.join("pdfium.dll.lib").exists()
    } else {
        true
    };

    if !runtime_lib_path.exists() || !import_lib_exists {
        tracing::debug!("Pdfium library not found, downloading for target: {}", target);
        tracing::debug!("Download URL: {}", download_url);
        download_and_extract_pdfium(&download_url, &pdfium_dir);
    } else {
        tracing::debug!("Pdfium library already present at {}", runtime_lib_path.display());
    }

    if target.contains("windows") {
        let lib_dir = pdfium_dir.join("lib");
        let dll_lib = lib_dir.join("pdfium.dll.lib");
        let expected_lib = lib_dir.join("pdfium.lib");

        if dll_lib.exists() && !expected_lib.exists() {
            tracing::debug!("Renaming cached {} to {}", dll_lib.display(), expected_lib.display());
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

    if repo.contains("bblanchon") {
        "7455".to_string()
    } else {
        "7469".to_string()
    }
}

fn get_pdfium_url_and_lib(target: &str) -> (String, String) {
    if target.contains("wasm") {
        let version = env::var("PDFIUM_WASM_VERSION")
            .ok()
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| get_latest_version("paulocoutinhox/pdfium-lib"));
        tracing::debug!("Using pdfium-lib version: {}", version);

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

    let version = env::var("PDFIUM_VERSION")
        .ok()
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| get_latest_version("bblanchon/pdfium-binaries"));
    tracing::debug!("Using pdfium-binaries version: {}", version);

    let url = format!(
        "https://github.com/bblanchon/pdfium-binaries/releases/download/chromium/{}/pdfium-{}-{}.tgz",
        version, platform, arch
    );

    (url, "pdfium".to_string())
}

fn download_and_extract_pdfium(url: &str, dest_dir: &Path) {
    use std::process::Command;

    fs::create_dir_all(dest_dir).expect("Failed to create pdfium directory");

    let archive_path = dest_dir.join("pdfium.tar.gz");

    tracing::debug!("Downloading Pdfium archive from: {}", url);
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

    let file_type = Command::new("file")
        .arg(archive_path.to_str().unwrap())
        .output()
        .expect("Failed to check file type");

    let file_type_output = String::from_utf8_lossy(&file_type.stdout);
    tracing::debug!("Downloaded file type: {}", file_type_output.trim());

    if !file_type_output.to_lowercase().contains("gzip") && !file_type_output.to_lowercase().contains("compressed") {
        fs::remove_file(&archive_path).ok();
        panic!(
            "Downloaded file is not a valid gzip archive. URL may be incorrect or version unavailable: {}",
            url
        );
    }

    tracing::debug!("Extracting Pdfium archive...");
    let status = Command::new("tar")
        .args(["-xzf", archive_path.to_str().unwrap(), "-C", dest_dir.to_str().unwrap()])
        .status()
        .expect("Failed to execute tar");

    if !status.success() {
        fs::remove_file(&archive_path).ok();
        panic!("Failed to extract Pdfium archive from {}", url);
    }

    fs::remove_file(&archive_path).ok();

    let target = env::var("TARGET").unwrap();
    if target.contains("windows") {
        let lib_dir = dest_dir.join("lib");
        let dll_lib = lib_dir.join("pdfium.dll.lib");
        let expected_lib = lib_dir.join("pdfium.lib");

        if dll_lib.exists() {
            tracing::debug!("Ensuring Windows import library at {}", expected_lib.display());
            if let Err(err) = fs::copy(&dll_lib, &expected_lib) {
                panic!("Failed to copy pdfium.dll.lib to pdfium.lib: {err}");
            }
        } else {
            tracing::debug!("Warning: Expected {} not found after extraction", dll_lib.display());
        }
    }

    tracing::debug!("Pdfium downloaded and extracted successfully");
}

fn copy_lib_to_package(pdfium_dir: &Path, target: &str) {
    let (runtime_lib_name, runtime_subdir) = runtime_library_info(target);
    let src_lib = pdfium_dir.join(runtime_subdir).join(&runtime_lib_name);

    if !src_lib.exists() {
        tracing::debug!("Source library not found: {}", src_lib.display());
        return;
    }

    // Fix install_name on macOS to use @rpath
    if target.contains("darwin") {
        fix_macos_install_name(&src_lib, &runtime_lib_name);
    }

    let crate_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let workspace_root = crate_dir.parent().unwrap().parent().unwrap();

    // Copy to target directory for CLI binary
    if let Ok(profile) = env::var("PROFILE") {
        let target_dir = workspace_root.join("target").join(profile);
        if target_dir.exists() {
            copy_lib_if_needed(&src_lib, &target_dir.join(&runtime_lib_name), "CLI target directory");
        }
    }

    let python_dest_dir = workspace_root.join("packages").join("python").join("kreuzberg");
    if python_dest_dir.exists() {
        copy_lib_if_needed(&src_lib, &python_dest_dir.join(&runtime_lib_name), "Python package");
    } else {
        tracing::debug!("Python package directory not found, skipping Python library copy");
    }

    let node_dest_dir = workspace_root.join("crates").join("kreuzberg-node");
    if node_dest_dir.exists() {
        copy_lib_if_needed(&src_lib, &node_dest_dir.join(&runtime_lib_name), "Node.js package");
    } else {
        tracing::debug!("Node.js package directory not found, skipping Node library copy");
    }

    let ruby_dest_dir = workspace_root.join("packages").join("ruby").join("lib");
    if ruby_dest_dir.exists() {
        copy_lib_if_needed(&src_lib, &ruby_dest_dir.join(&runtime_lib_name), "Ruby package");
    } else {
        tracing::debug!("Ruby package directory not found, skipping Ruby library copy");
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
            Ok(_) => tracing::debug!("Copied {} to {} ({})", src.display(), dest.display(), package_name),
            Err(e) => tracing::debug!("Failed to copy library to {}: {}", package_name, e),
        }
    }
}

fn runtime_library_info(target: &str) -> (String, &'static str) {
    if target.contains("windows") {
        ("pdfium.dll".to_string(), "bin")
    } else if target.contains("darwin") {
        ("libpdfium.dylib".to_string(), "lib")
    } else {
        ("libpdfium.so".to_string(), "lib")
    }
}

fn prepare_prebuilt_pdfium(prebuilt_src: &Path, dest_dir: &Path) -> io::Result<()> {
    if dest_dir.exists() {
        fs::remove_dir_all(dest_dir)?;
    }
    copy_dir_all(prebuilt_src, dest_dir)
}

fn copy_dir_all(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let target_path = dst.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_all(&entry.path(), &target_path)?;
        } else {
            fs::copy(entry.path(), &target_path)?;
        }
    }
    Ok(())
}

fn fix_macos_install_name(lib_path: &Path, lib_name: &str) {
    use std::process::Command;

    // Change install_name from ./libpdfium.dylib to @rpath/libpdfium.dylib
    let new_install_name = format!("@rpath/{}", lib_name);

    tracing::debug!("Fixing install_name for {} to {}", lib_path.display(), new_install_name);

    let status = Command::new("install_name_tool")
        .arg("-id")
        .arg(&new_install_name)
        .arg(lib_path)
        .status();

    match status {
        Ok(s) if s.success() => {
            tracing::debug!("Successfully updated install_name");
        }
        Ok(s) => {
            tracing::debug!("install_name_tool failed with status: {}", s);
        }
        Err(e) => {
            tracing::debug!("Failed to run install_name_tool: {}", e);
        }
    }
}
