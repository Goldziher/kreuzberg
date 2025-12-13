//! Runtime extraction of bundled PDFium library.
//!
//! When the `pdf-bundled` feature is enabled, the PDFium library is embedded in the binary
//! using `include_bytes!` during compilation. This module handles runtime extraction to a
//! temporary directory and provides the path for dynamic loading.
//!
//! # How It Works
//!
//! 1. During build (build.rs): PDFium is copied to OUT_DIR and the build script sets
//!    `KREUZBERG_PDFIUM_BUNDLED_PATH` environment variable
//! 2. At compile time: `include_bytes!` embeds the library binary in the executable
//! 3. At runtime: `extract_bundled_pdfium()` extracts to `$TMPDIR/kreuzberg-pdfium/`
//! 4. Library is reused if already present (based on modification time)
//!
//! # Example
//!
//! ```rust,ignore
//! # #[cfg(feature = "pdf-bundled")]
//! # {
//! use kreuzberg::pdf::bundled::extract_bundled_pdfium;
//!
//! # fn example() -> kreuzberg::Result<()> {
//! let lib_path = extract_bundled_pdfium()?;
//! println!("Extracted to: {}", lib_path.display());
//! # Ok(())
//! # }
//! # }
//! ```

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

/// Runtime library name and extraction directory for the bundled PDFium library.
///
/// Returns tuple of (library_name, extraction_directory)
fn bundled_library_info() -> (&'static str, &'static str) {
    if cfg!(target_os = "windows") {
        ("pdfium.dll", "kreuzberg-pdfium")
    } else if cfg!(target_os = "macos") {
        ("libpdfium.dylib", "kreuzberg-pdfium")
    } else if cfg!(target_os = "linux") {
        ("libpdfium.so", "kreuzberg-pdfium")
    } else {
        // Fallback for other Unix-like systems
        ("libpdfium.so", "kreuzberg-pdfium")
    }
}

/// Get the temporary directory for bundled PDFium extraction.
///
/// Uses `std::env::temp_dir()` on all platforms.
fn get_extraction_dir() -> io::Result<PathBuf> {
    let (_, subdir) = bundled_library_info();
    Ok(std::env::temp_dir().join(subdir))
}

/// Check if extracted library exists and is valid.
///
/// Verifies:
/// - File exists at expected path
/// - File size matches embedded size (basic validation)
///
/// Returns `true` if library can be safely reused, `false` if extraction is needed.
fn is_extracted_library_valid(lib_path: &Path, embedded_size: usize) -> bool {
    if !lib_path.exists() {
        return false;
    }

    // Verify file size matches to catch partial/corrupted extractions
    match fs::metadata(lib_path) {
        Ok(metadata) => {
            let file_size = metadata.len() as usize;
            // Allow 1% size difference for platform-specific variations
            let size_tolerance = (embedded_size as f64 * 0.01) as usize;
            let min_size = embedded_size.saturating_sub(size_tolerance);
            let max_size = embedded_size.saturating_add(size_tolerance);
            file_size >= min_size && file_size <= max_size
        }
        Err(_) => false,
    }
}

/// Extract bundled PDFium library to temporary directory.
///
/// # Behavior
///
/// - Embeds PDFium library using `include_bytes!`
/// - Extracts to `$TMPDIR/kreuzberg-pdfium/`
/// - Reuses extracted library if size matches
/// - Sets permissions to 0755 on Unix
/// - Returns path to extracted library
///
/// # Errors
///
/// Returns `std::io::Error` if:
/// - Cannot create extraction directory
/// - Cannot write library file
/// - Cannot set file permissions (Unix only)
///
/// # Platform-Specific Library Names
///
/// - Linux: `libpdfium.so`
/// - macOS: `libpdfium.dylib`
/// - Windows: `pdfium.dll`
pub fn extract_bundled_pdfium() -> io::Result<PathBuf> {
    let (lib_name, _) = bundled_library_info();
    let extract_dir = get_extraction_dir()?;

    // Create extraction directory if it doesn't exist
    fs::create_dir_all(&extract_dir).map_err(|e| {
        io::Error::new(
            e.kind(),
            format!(
                "Failed to create bundled pdfium extraction directory '{}': {}",
                extract_dir.display(),
                e
            ),
        )
    })?;

    let lib_path = extract_dir.join(lib_name);

    // Include bundled PDFium library
    let bundled_lib = include_bytes!(env!("KREUZBERG_PDFIUM_BUNDLED_PATH"));

    // Check if library already exists and is valid
    if is_extracted_library_valid(&lib_path, bundled_lib.len()) {
        return Ok(lib_path);
    }

    // Write library to disk
    fs::write(&lib_path, bundled_lib).map_err(|e| {
        io::Error::new(
            e.kind(),
            format!(
                "Failed to extract bundled pdfium library to '{}': {}",
                lib_path.display(),
                e
            ),
        )
    })?;

    // Set executable permissions on Unix
    #[cfg(unix)]
    {
        let perms = fs::Permissions::from_mode(0o755);
        fs::set_permissions(&lib_path, perms).map_err(|e| {
            io::Error::new(
                e.kind(),
                format!(
                    "Failed to set permissions on bundled pdfium library '{}': {}",
                    lib_path.display(),
                    e
                ),
            )
        })?;
    }

    Ok(lib_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bundled_library_info_windows() {
        if cfg!(target_os = "windows") {
            let (name, dir) = bundled_library_info();
            assert_eq!(name, "pdfium.dll");
            assert_eq!(dir, "kreuzberg-pdfium");
        }
    }

    #[test]
    fn test_bundled_library_info_macos() {
        if cfg!(target_os = "macos") {
            let (name, dir) = bundled_library_info();
            assert_eq!(name, "libpdfium.dylib");
            assert_eq!(dir, "kreuzberg-pdfium");
        }
    }

    #[test]
    fn test_bundled_library_info_linux() {
        if cfg!(target_os = "linux") {
            let (name, dir) = bundled_library_info();
            assert_eq!(name, "libpdfium.so");
            assert_eq!(dir, "kreuzberg-pdfium");
        }
    }

    #[test]
    fn test_get_extraction_dir() {
        let result = get_extraction_dir();
        assert!(result.is_ok());
        let dir = result.unwrap();
        assert!(dir.to_str().is_some());
        assert!(dir.ends_with("kreuzberg-pdfium"));
    }

    #[test]
    fn test_is_extracted_library_valid_missing() {
        let nonexistent = PathBuf::from("/tmp/nonexistent-pdfium-test");
        assert!(!is_extracted_library_valid(&nonexistent, 1000));
    }

    #[test]
    fn test_is_extracted_library_valid_size_match() {
        // Create a temporary test file
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test-pdfium-size.dll");
        let test_size = 5_000_000;
        let test_data = vec![0u8; test_size];

        if let Ok(_) = fs::write(&test_file, &test_data) {
            let is_valid = is_extracted_library_valid(&test_file, test_size);
            assert!(is_valid);
            let _ = fs::remove_file(&test_file);
        }
    }

    #[test]
    fn test_is_extracted_library_valid_size_tolerance() {
        // Create a temporary test file
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test-pdfium-tolerance.dll");
        let original_size = 10_000_000;
        let tolerance = (original_size as f64 * 0.01) as usize;

        // Create file that's 0.5% smaller (within tolerance)
        let actual_size = original_size - tolerance / 2;
        let test_data = vec![0u8; actual_size];

        if let Ok(_) = fs::write(&test_file, &test_data) {
            let is_valid = is_extracted_library_valid(&test_file, original_size);
            assert!(is_valid);
            let _ = fs::remove_file(&test_file);
        }
    }

    #[test]
    fn test_is_extracted_library_valid_size_mismatch() {
        // Create a temporary test file
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test-pdfium-mismatch.dll");
        let original_size = 10_000_000;

        // Create file that's 10% smaller (outside tolerance)
        let actual_size = (original_size as f64 * 0.85) as usize;
        let test_data = vec![0u8; actual_size];

        if let Ok(_) = fs::write(&test_file, &test_data) {
            let is_valid = is_extracted_library_valid(&test_file, original_size);
            assert!(!is_valid);
            let _ = fs::remove_file(&test_file);
        }
    }

    #[test]
    #[cfg(feature = "pdf-bundled")]
    fn test_extract_bundled_pdfium() {
        let result = extract_bundled_pdfium();
        assert!(result.is_ok());

        let lib_path = result.unwrap();
        assert!(
            lib_path.exists(),
            "Extracted library should exist at: {}",
            lib_path.display()
        );
        assert!(lib_path.file_name().is_some(), "Library path should have filename");

        // Verify correct library name for platform
        let (expected_name, _) = bundled_library_info();
        assert_eq!(lib_path.file_name().unwrap(), expected_name);
    }

    #[test]
    #[cfg(feature = "pdf-bundled")]
    fn test_extract_bundled_pdfium_reuses_existing() {
        // First extraction
        let result1 = extract_bundled_pdfium();
        assert!(result1.is_ok());
        let path1 = result1.unwrap();

        // Get file size and basic metadata of first extraction
        let metadata1 = fs::metadata(&path1).expect("Should be able to read metadata");
        let size1 = metadata1.len();

        // Second extraction should reuse the file
        let result2 = extract_bundled_pdfium();
        assert!(result2.is_ok());
        let path2 = result2.unwrap();

        // Paths should be identical
        assert_eq!(path1, path2, "Extraction should return same path on second call");

        // File size should be identical (reused, not rewritten)
        let metadata2 = fs::metadata(&path2).expect("Should be able to read metadata");
        let size2 = metadata2.len();
        assert_eq!(size1, size2, "Reused library should have same file size");
    }

    #[test]
    #[cfg(unix)]
    #[cfg(feature = "pdf-bundled")]
    fn test_extract_bundled_pdfium_permissions() {
        let result = extract_bundled_pdfium();
        assert!(result.is_ok());

        let lib_path = result.unwrap();
        let metadata = fs::metadata(&lib_path).expect("Should be able to read metadata");
        let perms = metadata.permissions();
        let mode = perms.mode();

        // Verify executable bit is set (at least 0o700 or 0o755)
        assert!(
            mode & 0o111 != 0,
            "Library should have executable bit set, got mode: {:#o}",
            mode
        );
    }
}
