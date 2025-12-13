use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::Duration;

/// PDFium linking strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PdfiumLinkStrategy {
    /// Download and link dynamically (default behavior)
    DownloadDynamic,
    /// Download and link statically (pdf-static feature)
    DownloadStatic,
    /// Download, link dynamically, and embed in binary (pdf-bundled feature)
    Bundled,
    /// Use system-installed pdfium via pkg-config (pdf-system feature)
    System,
}

// ============================================================================
// MAIN BUILD ORCHESTRATION
// ============================================================================

fn main() {
    let target = env::var("TARGET").unwrap();
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    println!("cargo::rustc-check-cfg=cfg(coverage)");

    // Skip pdfium linking if the pdf feature is not enabled
    if !cfg!(feature = "pdf") {
        tracing::debug!("PDF feature not enabled, skipping pdfium linking");
        return;
    }

    validate_feature_exclusivity();
    let strategy = determine_link_strategy(&target);

    tracing::debug!("Using PDFium linking strategy: {:?}", strategy);

    match strategy {
        PdfiumLinkStrategy::DownloadDynamic => {
            let pdfium_dir = download_or_use_prebuilt(&target, &out_dir);
            link_dynamically(&pdfium_dir, &target);
            copy_lib_to_package(&pdfium_dir, &target);
        }
        PdfiumLinkStrategy::DownloadStatic => {
            let pdfium_dir = download_or_use_prebuilt(&target, &out_dir);
            link_statically(&pdfium_dir, &target);
            // Skip copy_lib_to_package - library embedded in binary
        }
        PdfiumLinkStrategy::Bundled => {
            let pdfium_dir = download_or_use_prebuilt(&target, &out_dir);
            link_bundled(&pdfium_dir, &target, &out_dir);
            // Skip copy_lib_to_package - each binary extracts its own
        }
        PdfiumLinkStrategy::System => {
            link_system(&target);
            // No download or copy needed
        }
    }

    link_system_frameworks(&target);
    println!("cargo:rerun-if-changed=build.rs");
}

// ============================================================================
// FEATURE & STRATEGY VALIDATION
// ============================================================================

/// Validate that only one linking strategy feature is enabled at a time
fn validate_feature_exclusivity() {
    let strategies = [
        cfg!(feature = "pdf-static"),
        cfg!(feature = "pdf-bundled"),
        cfg!(feature = "pdf-system"),
    ];
    let count = strategies.iter().filter(|&&x| x).count();

    if count > 1 {
        panic!(
            "Only one of pdf-static, pdf-bundled, pdf-system can be enabled at once.\n\
             Please choose a single PDFium linking strategy."
        );
    }
}

/// Determine which linking strategy to use based on features and target
fn determine_link_strategy(target: &str) -> PdfiumLinkStrategy {
    // WASM always uses static linking
    if target.contains("wasm") {
        return PdfiumLinkStrategy::DownloadStatic;
    }

    // Feature-based strategy selection (priority order)
    if cfg!(feature = "pdf-system") {
        return PdfiumLinkStrategy::System;
    }
    if cfg!(feature = "pdf-bundled") {
        return PdfiumLinkStrategy::Bundled;
    }
    if cfg!(feature = "pdf-static") {
        return PdfiumLinkStrategy::DownloadStatic;
    }

    // Default: download and link dynamically
    PdfiumLinkStrategy::DownloadDynamic
}

// ============================================================================
// DOWNLOAD & PREBUILT ORCHESTRATION
// ============================================================================

/// Download PDFium or use prebuilt directory
///
/// This is the main orchestrator function that:
/// 1. Checks for `KREUZBERG_PDFIUM_PREBUILT` environment variable
/// 2. If set and valid, uses prebuilt pdfium directory
/// 3. If not set, downloads pdfium to out_dir (with caching)
/// 4. Returns PathBuf to pdfium directory
///
/// Reuses all existing helper functions:
/// - `get_pdfium_url_and_lib()` - determines download URL for target
/// - `download_and_extract_pdfium()` - downloads with retry logic
/// - `runtime_library_info()` - platform-specific library names
/// - `prepare_prebuilt_pdfium()` - handles prebuilt copy
fn download_or_use_prebuilt(target: &str, out_dir: &Path) -> PathBuf {
    let (download_url, _lib_name) = get_pdfium_url_and_lib(target);
    let pdfium_dir = out_dir.join("pdfium");

    // Check for prebuilt pdfium directory
    if let Some(prebuilt) = env::var_os("KREUZBERG_PDFIUM_PREBUILT") {
        let prebuilt_path = PathBuf::from(prebuilt);
        if prebuilt_path.exists() {
            prepare_prebuilt_pdfium(&prebuilt_path, &pdfium_dir)
                .unwrap_or_else(|err| panic!("Failed to copy Pdfium from {}: {}", prebuilt_path.display(), err));
            return pdfium_dir;
        } else {
            panic!(
                "Environment variable KREUZBERG_PDFIUM_PREBUILT points to '{}' but the directory does not exist",
                prebuilt_path.display()
            );
        }
    }

    // Check if library already exists (cache validation)
    let (runtime_lib_name, runtime_subdir) = runtime_library_info(target);
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

    // Windows-specific: ensure pdfium.lib exists
    if target.contains("windows") {
        let lib_dir = pdfium_dir.join("lib");
        let dll_lib = lib_dir.join("pdfium.dll.lib");
        let expected_lib = lib_dir.join("pdfium.lib");

        if dll_lib.exists() && !expected_lib.exists() {
            tracing::debug!("Renaming cached {} to {}", dll_lib.display(), expected_lib.display());
            fs::rename(&dll_lib, &expected_lib).expect("Failed to rename pdfium.dll.lib to pdfium.lib");
        }
    }

    pdfium_dir
}

// ============================================================================
// DOWNLOAD UTILITIES
// ============================================================================

/// Fetch the latest release version from a GitHub repository
///
/// Uses curl to query the GitHub API and extract the tag_name from the
/// latest release JSON response. Falls back to "7529" if API call fails.
fn get_latest_version(repo: &str) -> String {
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

    "7529".to_string()
}

/// Get the download URL and library name for the target platform
///
/// Determines platform/architecture from target triple and constructs
/// the appropriate GitHub release download URL. Supports:
/// - WASM: paulocoutinhox/pdfium-lib
/// - Other platforms: bblanchon/pdfium-binaries
fn get_pdfium_url_and_lib(target: &str) -> (String, String) {
    if target.contains("wasm") {
        let version = env::var("PDFIUM_WASM_VERSION")
            .ok()
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| get_latest_version("paulocoutinhox/pdfium-lib"));
        tracing::debug!("Using pdfium-lib version: {}", version);

        // WASM builds use a single 'wasm.tgz' asset regardless of architecture
        // The archive contains both wasm32 and wasm64 if available
        return (
            format!(
                "https://github.com/paulocoutinhox/pdfium-lib/releases/download/{}/wasm.tgz",
                version
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

/// Download and extract PDFium archive with retry logic
///
/// Features:
/// - Exponential backoff retry (configurable via env vars)
/// - File type validation (gzip check)
/// - Windows-specific import library handling (pdfium.dll.lib -> pdfium.lib)
/// - Environment variables:
///   - KREUZBERG_PDFIUM_DOWNLOAD_RETRIES: number of retries (default: 5)
///   - KREUZBERG_PDFIUM_DOWNLOAD_BACKOFF_SECS: initial backoff in seconds (default: 2)
fn download_and_extract_pdfium(url: &str, dest_dir: &Path) {
    fs::create_dir_all(dest_dir).expect("Failed to create pdfium directory");

    let archive_path = dest_dir.join("pdfium.tar.gz");
    let retries = env::var("KREUZBERG_PDFIUM_DOWNLOAD_RETRIES")
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(5);
    let base_delay = env::var("KREUZBERG_PDFIUM_DOWNLOAD_BACKOFF_SECS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(2);

    let archive_path_str = archive_path
        .to_str()
        .unwrap_or_else(|| panic!("Non-UTF8 path for archive: {}", archive_path.display()));
    let mut last_error = String::new();

    for attempt in 1..=retries {
        let _ = fs::remove_file(&archive_path);
        tracing::debug!(
            "Downloading Pdfium archive from: {} (attempt {}/{})",
            url,
            attempt,
            retries
        );

        let status = Command::new("curl")
            .args(["-f", "-L", "-o", archive_path_str, url])
            .status();

        match status {
            Ok(code) if code.success() => {
                last_error.clear();
                break;
            }
            Ok(code) => {
                last_error = format!("curl exited with {:?}", code.code());
            }
            Err(err) => {
                last_error = format!("failed to spawn curl: {err}");
            }
        }

        if attempt == retries {
            panic!(
                "Failed to download Pdfium from {} after {} attempts. Last error: {}",
                url, retries, last_error
            );
        }

        let exponent = u32::min(attempt, 5);
        let multiplier = 1u64 << exponent;
        let delay_secs = base_delay.saturating_mul(multiplier).min(30);
        println!(
            "cargo:warning=Pdfium download failed (attempt {}/{}) - {}. Retrying in {}s",
            attempt, retries, last_error, delay_secs
        );
        thread::sleep(Duration::from_secs(delay_secs));
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

// ============================================================================
// PREBUILT HANDLING
// ============================================================================

/// Prepare prebuilt PDFium by copying to destination directory
///
/// Removes existing destination if present, then recursively copies
/// all files from prebuilt source to destination.
fn prepare_prebuilt_pdfium(prebuilt_src: &Path, dest_dir: &Path) -> io::Result<()> {
    if dest_dir.exists() {
        fs::remove_dir_all(dest_dir)?;
    }
    copy_dir_all(prebuilt_src, dest_dir)
}

/// Recursively copy directory tree
///
/// Used by `prepare_prebuilt_pdfium()` to copy entire pdfium directory
/// structure, preserving all files and subdirectories.
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

// ============================================================================
// PLATFORM UTILITIES
// ============================================================================

/// Get platform-specific runtime library name and subdirectory
///
/// Returns tuple of (library_name, subdirectory) for the target platform:
/// - WASM: ("libpdfium.a", "lib")
/// - Windows: ("pdfium.dll", "bin")
/// - macOS: ("libpdfium.dylib", "lib")
/// - Linux: ("libpdfium.so", "lib")
fn runtime_library_info(target: &str) -> (String, &'static str) {
    if target.contains("wasm") {
        ("libpdfium.a".to_string(), "lib")
    } else if target.contains("windows") {
        ("pdfium.dll".to_string(), "bin")
    } else if target.contains("darwin") {
        ("libpdfium.dylib".to_string(), "lib")
    } else {
        ("libpdfium.so".to_string(), "lib")
    }
}

/// Fix macOS install name (rpath) for dynamic library
///
/// Uses install_name_tool to set the install name to @rpath/{lib_name}
/// to enable relative path loading on macOS.
fn fix_macos_install_name(lib_path: &Path, lib_name: &str) {
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

/// Code sign binary on macOS if needed
///
/// Uses codesign to sign the binary. Identity from KREUZBERG_CODESIGN_IDENTITY
/// env var (default: "-" for adhoc signing). Only runs on apple-darwin targets.
fn codesign_if_needed(target: &str, binary: &Path) {
    if !target.contains("apple-darwin") || !binary.exists() {
        return;
    }

    let identity = env::var("KREUZBERG_CODESIGN_IDENTITY").unwrap_or_else(|_| "-".to_string());
    let status = Command::new("codesign")
        .arg("--force")
        .arg("--timestamp=none")
        .arg("--sign")
        .arg(identity)
        .arg(binary)
        .status();

    match status {
        Ok(result) if result.success() => {
            tracing::debug!("Codesigned {}", binary.display());
        }
        Ok(result) => {
            tracing::debug!(
                "codesign exited with status {} while signing {}",
                result,
                binary.display()
            );
        }
        Err(err) => {
            tracing::debug!("Failed to run codesign for {}: {}", binary.display(), err);
        }
    }
}

// ============================================================================
// LINKING STRATEGIES
// ============================================================================

/// Link PDFium dynamically (default)
///
/// Sets up linker to use PDFium as a dynamic library (.dylib/.so/.dll)
/// with platform-specific rpath configuration for runtime library discovery.
fn link_dynamically(pdfium_dir: &Path, target: &str) {
    let lib_dir = pdfium_dir.join("lib");
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-lib=dylib=pdfium");

    // Set rpath for dynamic linking
    if target.contains("darwin") {
        println!("cargo:rustc-link-arg=-Wl,-rpath,@loader_path");
        println!("cargo:rustc-link-arg=-Wl,-rpath,@loader_path/.");
    } else if target.contains("linux") {
        println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN");
        println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN/.");
    }
}

/// Link PDFium statically (pdf-static feature)
///
/// Embeds PDFium into the binary as a static library. Adds system
/// dependencies required for static linking on Linux.
fn link_statically(pdfium_dir: &Path, target: &str) {
    let lib_dir = pdfium_dir.join("lib");
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-lib=static=pdfium");

    // Static linking requires additional system dependencies
    if target.contains("linux") {
        // Linux requires additional libraries for static linking
        println!("cargo:rustc-link-lib=dylib=pthread");
        println!("cargo:rustc-link-lib=dylib=dl");
    }
}

/// Link PDFium bundled (pdf-bundled feature)
///
/// Links dynamically but copies library to OUT_DIR for embedding in binary.
/// Each binary extracts and uses its own copy of the PDFium library.
fn link_bundled(pdfium_dir: &Path, target: &str, out_dir: &Path) {
    // Link dynamically for build
    link_dynamically(pdfium_dir, target);

    // Copy library to OUT_DIR for bundling
    let (runtime_lib_name, runtime_subdir) = runtime_library_info(target);
    let src_lib = pdfium_dir.join(runtime_subdir).join(&runtime_lib_name);
    let bundled_lib = out_dir.join(&runtime_lib_name);

    if src_lib.exists() {
        fs::copy(&src_lib, &bundled_lib)
            .unwrap_or_else(|err| panic!("Failed to copy library to OUT_DIR for bundling: {}", err));

        // Emit environment variable with bundled library path
        let bundled_path = bundled_lib
            .to_str()
            .unwrap_or_else(|| panic!("Non-UTF8 path for bundled library: {}", bundled_lib.display()));
        println!("cargo:rustc-env=KREUZBERG_PDFIUM_BUNDLED_PATH={}", bundled_path);

        tracing::debug!("Bundled PDFium library at: {}", bundled_path);
    } else {
        panic!("Cannot bundle PDFium: library not found at {}", src_lib.display());
    }
}

/// Link system-installed PDFium (pdf-system feature)
///
/// Attempts to find PDFium via pkg-config first, then falls back to
/// environment variables (KREUZBERG_PDFIUM_SYSTEM_PATH, KREUZBERG_PDFIUM_SYSTEM_INCLUDE).
fn link_system(_target: &str) {
    // Try pkg-config first
    match pkg_config::Config::new().atleast_version("5.0").probe("pdfium") {
        Ok(library) => {
            tracing::debug!("Found system pdfium via pkg-config");
            for include_path in &library.include_paths {
                println!("cargo:include={}", include_path.display());
            }
            return;
        }
        Err(err) => {
            tracing::debug!("pkg-config probe failed: {}", err);
        }
    }

    // Fallback to environment variables
    let lib_path = env::var("KREUZBERG_PDFIUM_SYSTEM_PATH").ok();
    let include_path = env::var("KREUZBERG_PDFIUM_SYSTEM_INCLUDE").ok();

    if let Some(lib_dir) = lib_path {
        let lib_dir_path = PathBuf::from(&lib_dir);
        if !lib_dir_path.exists() {
            panic!(
                "KREUZBERG_PDFIUM_SYSTEM_PATH points to '{}' but the directory does not exist",
                lib_dir
            );
        }

        println!("cargo:rustc-link-search=native={}", lib_dir);
        println!("cargo:rustc-link-lib=dylib=pdfium");

        if let Some(inc_dir) = include_path {
            println!("cargo:include={}", inc_dir);
        }

        tracing::debug!("Using system pdfium from: {}", lib_dir);
        return;
    }

    // No system pdfium found
    panic!(
        "pdf-system feature enabled but pdfium not found.\n\
         \n\
         Please install pdfium system-wide or provide:\n\
         - KREUZBERG_PDFIUM_SYSTEM_PATH: path to directory containing libpdfium\n\
         - KREUZBERG_PDFIUM_SYSTEM_INCLUDE: path to pdfium headers (optional)\n\
         \n\
         Alternatively, use a different linking strategy:\n\
         - Default (dynamic): cargo build --features pdf\n\
         - Static linking: cargo build --features pdf,pdf-static\n\
         - Bundled: cargo build --features pdf,pdf-bundled"
    );
}

/// Link system frameworks and standard libraries
///
/// Adds platform-specific system libraries required for PDFium linking:
/// - macOS: CoreFoundation, CoreGraphics, CoreText, AppKit, libc++
/// - Linux: stdc++, libm
/// - Windows: gdi32, user32, advapi32
fn link_system_frameworks(target: &str) {
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
}

// ============================================================================
// LIBRARY DISTRIBUTION
// ============================================================================

/// Copy PDFium library to various package directories
///
/// Distributes the compiled/downloaded PDFium library to:
/// - CLI target directories (debug/release)
/// - Python package directory
/// - Node.js package directory
/// - Ruby gem directory
///
/// On macOS, also fixes install_name and applies code signing.
fn copy_lib_to_package(pdfium_dir: &Path, target: &str) {
    let (runtime_lib_name, runtime_subdir) = runtime_library_info(target);
    let src_lib = pdfium_dir.join(runtime_subdir).join(&runtime_lib_name);

    if !src_lib.exists() {
        tracing::debug!("Source library not found: {}", src_lib.display());
        return;
    }

    if target.contains("darwin") {
        fix_macos_install_name(&src_lib, &runtime_lib_name);
        codesign_if_needed(target, &src_lib);
    }

    let crate_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let workspace_root = crate_dir.parent().unwrap().parent().unwrap();

    if let Ok(profile) = env::var("PROFILE") {
        let target_dir = if let Ok(cargo_target) = env::var("TARGET") {
            workspace_root.join("target").join(cargo_target).join(&profile)
        } else {
            workspace_root.join("target").join(&profile)
        };

        if target_dir.exists() {
            copy_lib_if_needed(
                &src_lib,
                &target_dir.join(&runtime_lib_name),
                "CLI target directory",
                target,
            );
        }

        let simple_target_dir = workspace_root.join("target").join(&profile);
        if simple_target_dir != target_dir {
            fs::create_dir_all(&simple_target_dir).ok();
            copy_lib_if_needed(
                &src_lib,
                &simple_target_dir.join(&runtime_lib_name),
                "Java FFI target directory",
                target,
            );
        }
    }

    let python_dest_dir = workspace_root.join("packages").join("python").join("kreuzberg");
    if python_dest_dir.exists() {
        copy_lib_if_needed(
            &src_lib,
            &python_dest_dir.join(&runtime_lib_name),
            "Python package",
            target,
        );
    } else {
        tracing::debug!("Python package directory not found, skipping Python library copy");
    }

    let node_dest_dir = workspace_root.join("crates").join("kreuzberg-node");
    if node_dest_dir.exists() {
        copy_lib_if_needed(
            &src_lib,
            &node_dest_dir.join(&runtime_lib_name),
            "Node.js package",
            target,
        );
    } else {
        tracing::debug!("Node.js package directory not found, skipping Node library copy");
    }

    let ruby_dest_dir = workspace_root.join("packages").join("ruby").join("lib");
    if ruby_dest_dir.exists() {
        copy_lib_if_needed(&src_lib, &ruby_dest_dir.join(&runtime_lib_name), "Ruby package", target);
    } else {
        tracing::debug!("Ruby package directory not found, skipping Ruby library copy");
    }
}

/// Copy library to destination if needed (based on modification time)
///
/// Only copies if destination doesn't exist or source is newer than destination.
/// Applies platform-specific post-processing (code signing on macOS).
fn copy_lib_if_needed(src: &Path, dest: &Path, package_name: &str, target: &str) {
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
            Ok(_) => {
                tracing::debug!("Copied {} to {} ({})", src.display(), dest.display(), package_name);
                codesign_if_needed(target, dest);
            }
            Err(e) => tracing::debug!("Failed to copy library to {}: {}", package_name, e),
        }
    }
}
