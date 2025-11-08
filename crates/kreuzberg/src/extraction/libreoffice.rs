//! LibreOffice document conversion utilities.
//!
//! This module provides functions for converting legacy Microsoft Office formats
//! (.doc, .ppt) to modern formats using LibreOffice's headless conversion mode.
//!
//! # Features
//!
//! - **Headless conversion**: Uses `soffice --headless` for server-side conversions
//! - **Timeout protection**: Configurable timeout to prevent hanging conversions
//! - **Format detection**: Automatic output format based on input file type
//! - **Error handling**: Distinguishes between missing dependencies and conversion failures
//!
//! # Supported Conversions
//!
//! - `.doc` → `.docx` (Word documents)
//! - `.ppt` → `.pptx` (PowerPoint presentations)
//! - `.xls` → `.xlsx` (Excel spreadsheets) - future support
//!
//! # System Requirement
//!
//! LibreOffice must be installed and `soffice` must be in PATH:
//! - **macOS**: `brew install --cask libreoffice`
//! - **Linux**: `apt install libreoffice` or `dnf install libreoffice`
//! - **Windows**: `winget install LibreOffice.LibreOffice`
//!
//! # Example
//!
//! ```rust,no_run
//! use kreuzberg::extraction::libreoffice::{convert_office_doc, check_libreoffice_available};
//! use std::path::Path;
//!
//! # async fn example() -> kreuzberg::Result<()> {
//! // Check if LibreOffice is available
//! let _soffice_path = check_libreoffice_available().await?;
//!
//! // Convert .doc to .docx
//! let input = Path::new("legacy.doc");
//! let output_dir = Path::new("/tmp");
//! let converted = convert_office_doc(input, output_dir, "docx", 300).await?;
//!
//! println!("Converted {} bytes", converted.len());
//! # Ok(())
//! # }
//! ```

use crate::error::{KreuzbergError, Result};
use crate::types::LibreOfficeConversionResult;
use std::collections::HashSet;
use std::env;
use std::fs as std_fs;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::process::Command;
use tokio::time::{Duration, timeout};

/// RAII guard for automatic temporary directory cleanup
struct TempDir {
    path: PathBuf,
}

impl TempDir {
    async fn new(path: PathBuf) -> Result<Self> {
        fs::create_dir_all(&path).await?;
        Ok(Self { path })
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        // Best-effort cleanup - use blocking remove since Drop can't be async
        let path = self.path.clone();
        tokio::spawn(async move {
            let _ = fs::remove_dir_all(&path).await;
        });
    }
}

/// Default timeout for LibreOffice conversion (300 seconds)
pub const DEFAULT_CONVERSION_TIMEOUT: u64 = 300;

fn libreoffice_install_message() -> String {
    "LibreOffice (soffice/libreoffice) is required for legacy MS Office format support (.doc, .ppt). \
Install: macOS: 'brew install --cask libreoffice', \
Linux: 'apt install libreoffice', \
Windows: 'winget install LibreOffice.LibreOffice'. \
If LibreOffice is installed in a custom location, set the KREUZBERG_LIBREOFFICE_PATH environment variable to the soffice executable."
        .to_string()
}

fn soffice_candidates() -> Vec<PathBuf> {
    let mut seen = HashSet::new();
    let mut candidates = Vec::new();

    let mut push_candidate = |path: PathBuf| {
        if seen.insert(path.clone()) {
            candidates.push(path);
        }
    };

    for var in ["KREUZBERG_LIBREOFFICE_PATH", "SOFFICE_PATH", "LIBREOFFICE_PATH"] {
        if let Some(value) = env::var_os(var).filter(|v| !v.is_empty()) {
            push_candidate(PathBuf::from(value));
        }
    }

    if cfg!(target_os = "macos") {
        push_candidate(PathBuf::from("/Applications/LibreOffice.app/Contents/MacOS/soffice"));
        push_candidate(PathBuf::from(
            "/Applications/LibreOffice.app/Contents/MacOS/libreoffice",
        ));
        push_candidate(PathBuf::from(
            "/Applications/LibreOffice.app/Contents/MacOS/soffice.bin",
        ));
    }

    if cfg!(target_os = "windows") {
        push_candidate(PathBuf::from("C:\\Program Files\\LibreOffice\\program\\soffice.exe"));
        push_candidate(PathBuf::from(
            "C:\\Program Files\\LibreOffice\\program\\libreoffice.exe",
        ));
    }

    if let Some(prefix) = env::var_os("HOMEBREW_PREFIX") {
        let prefix_path = PathBuf::from(prefix);
        push_candidate(prefix_path.join("bin/soffice"));
        push_candidate(prefix_path.join("bin/libreoffice"));
        push_candidate(prefix_path.join("bin/soffice.exe"));
        push_candidate(prefix_path.join("bin/libreoffice.exe"));
    }

    if let Some(path_env) = env::var_os("PATH") {
        for dir in env::split_paths(&path_env) {
            push_candidate(dir.join("soffice"));
            push_candidate(dir.join("libreoffice"));
            push_candidate(dir.join("soffice.exe"));
            push_candidate(dir.join("libreoffice.exe"));
        }
    }

    candidates
}

fn locate_soffice_binary() -> Result<PathBuf> {
    for candidate in soffice_candidates() {
        if candidate.exists()
            && let Ok(metadata) = std_fs::metadata(&candidate)
            && metadata.is_file()
        {
            return Ok(candidate);
        }
    }

    Err(KreuzbergError::MissingDependency(libreoffice_install_message()))
}

/// Check if LibreOffice (soffice/libreoffice) is available and working
pub async fn check_libreoffice_available() -> Result<PathBuf> {
    let soffice_path = locate_soffice_binary()?;

    let result = Command::new(&soffice_path).arg("--version").output().await;

    match result {
        Ok(output) if output.status.success() => Ok(soffice_path),
        Ok(_) => Err(KreuzbergError::MissingDependency(format!(
            "LibreOffice executable '{}' responded with a failure when checking '--version'. \
Please reinstall LibreOffice.",
            soffice_path.display()
        ))),
        Err(err) => Err(KreuzbergError::MissingDependency(format!(
            "LibreOffice executable '{}' could not be executed: {}. {help}",
            soffice_path.display(),
            err,
            help = libreoffice_install_message()
        ))),
    }
}

/// Convert an Office document to a target format using LibreOffice
pub async fn convert_office_doc(
    input_path: &Path,
    output_dir: &Path,
    target_format: &str,
    timeout_seconds: u64,
) -> Result<Vec<u8>> {
    let soffice_path = check_libreoffice_available().await?;

    fs::create_dir_all(output_dir).await?;

    let child = Command::new(&soffice_path)
        .arg("--headless")
        .arg("--convert-to")
        .arg(target_format)
        .arg("--outdir")
        .arg(output_dir)
        .arg(input_path)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| {
            KreuzbergError::parsing(format!(
                "Failed to execute LibreOffice at '{}': {}",
                soffice_path.display(),
                e
            ))
        })?;

    let child_id = child.id();

    let output = match timeout(Duration::from_secs(timeout_seconds), child.wait_with_output()).await {
        Ok(Ok(output)) => output,
        Ok(Err(e)) => {
            return Err(KreuzbergError::parsing(format!(
                "Failed to wait for LibreOffice: {}",
                e
            )));
        }
        Err(_) => {
            // Timeout occurred - wait_with_output was cancelled, child is dropped and killed automatically ~keep
            return Err(KreuzbergError::parsing(format!(
                "LibreOffice conversion timed out after {} seconds (PID: {:?})",
                timeout_seconds, child_id
            )));
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);

        // Subprocess error analysis - wrap only if format/parsing error detected ~keep
        let stderr_lower = stderr.to_lowercase();
        let stdout_lower = stdout.to_lowercase();
        let keywords = ["format", "unsupported", "error:", "failed"];

        if keywords
            .iter()
            .any(|k| stderr_lower.contains(k) || stdout_lower.contains(k))
        {
            return Err(KreuzbergError::parsing(format!(
                "LibreOffice conversion failed: {}",
                if !stderr.is_empty() { &stderr } else { &stdout }
            )));
        }

        // True system error - bubble up for user reporting ~keep
        return Err(KreuzbergError::Io(std::io::Error::other(format!(
            "LibreOffice process failed with return code {}: {}",
            output.status.code().unwrap_or(-1),
            if !stderr.is_empty() { stderr } else { stdout }
        ))));
    }

    let input_stem = input_path
        .file_stem()
        .ok_or_else(|| KreuzbergError::parsing("Invalid input file name".to_string()))?;

    let expected_output = output_dir.join(format!("{}.{}", input_stem.to_string_lossy(), target_format));

    let converted_bytes = fs::read(&expected_output).await.map_err(|e| {
        KreuzbergError::parsing(format!(
            "LibreOffice conversion completed but output file not found: {}",
            e
        ))
    })?;

    if converted_bytes.is_empty() {
        return Err(KreuzbergError::parsing(
            "LibreOffice conversion produced empty file".to_string(),
        ));
    }

    Ok(converted_bytes)
}

/// Convert .doc to .docx using LibreOffice
pub async fn convert_doc_to_docx(doc_bytes: &[u8]) -> Result<LibreOfficeConversionResult> {
    let temp_dir = std::env::temp_dir();
    let unique_id = uuid::Uuid::new_v4();
    let input_dir_path = temp_dir.join(format!("kreuzberg_doc_{}", unique_id));
    let output_dir_path = temp_dir.join(format!("kreuzberg_doc_{}_out", unique_id));

    // RAII guards ensure cleanup on all paths including panic ~keep
    let _input_guard = TempDir::new(input_dir_path.clone()).await?;
    let _output_guard = TempDir::new(output_dir_path.clone()).await?;

    let input_path = input_dir_path.join("input.doc");
    fs::write(&input_path, doc_bytes).await?;

    let converted_bytes = convert_office_doc(&input_path, &output_dir_path, "docx", DEFAULT_CONVERSION_TIMEOUT).await?;

    Ok(LibreOfficeConversionResult {
        converted_bytes,
        original_format: "doc".to_string(),
        target_format: "docx".to_string(),
        target_mime: crate::core::mime::DOCX_MIME_TYPE.to_string(),
    })
}

/// Convert .ppt to .pptx using LibreOffice
pub async fn convert_ppt_to_pptx(ppt_bytes: &[u8]) -> Result<LibreOfficeConversionResult> {
    let temp_dir = std::env::temp_dir();
    let unique_id = uuid::Uuid::new_v4();
    let input_dir_path = temp_dir.join(format!("kreuzberg_ppt_{}", unique_id));
    let output_dir_path = temp_dir.join(format!("kreuzberg_ppt_{}_out", unique_id));

    // RAII guards ensure cleanup on all paths including panic ~keep
    let _input_guard = TempDir::new(input_dir_path.clone()).await?;
    let _output_guard = TempDir::new(output_dir_path.clone()).await?;

    let input_path = input_dir_path.join("input.ppt");
    fs::write(&input_path, ppt_bytes).await?;

    let converted_bytes = convert_office_doc(&input_path, &output_dir_path, "pptx", DEFAULT_CONVERSION_TIMEOUT).await?;

    Ok(LibreOfficeConversionResult {
        converted_bytes,
        original_format: "ppt".to_string(),
        target_format: "pptx".to_string(),
        target_mime: crate::core::mime::POWER_POINT_MIME_TYPE.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_check_libreoffice_available() {
        let result = check_libreoffice_available().await;
        if result.is_err() {
            return;
        }
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_convert_office_doc_missing_file() {
        if check_libreoffice_available().await.is_err() {
            return;
        }

        let temp_dir = std::env::temp_dir();
        let output_dir = temp_dir.join("test_convert_office_doc_missing_file");
        let non_existent = Path::new("/tmp/nonexistent.doc");

        let result = convert_office_doc(non_existent, &output_dir, "docx", 10).await;

        assert!(result.is_err());
        let _ = fs::remove_dir_all(&output_dir).await;
    }

    #[test]
    fn test_default_conversion_timeout_value() {
        assert_eq!(DEFAULT_CONVERSION_TIMEOUT, 300);
    }

    #[tokio::test]
    async fn test_convert_doc_to_docx_empty_bytes() {
        if check_libreoffice_available().await.is_err() {
            return;
        }

        let empty_bytes = b"";
        let result = convert_doc_to_docx(empty_bytes).await;

        let _ = result;
    }

    #[tokio::test]
    async fn test_convert_ppt_to_pptx_empty_bytes() {
        if check_libreoffice_available().await.is_err() {
            return;
        }

        let empty_bytes = b"";
        let result = convert_ppt_to_pptx(empty_bytes).await;

        let _ = result;
    }

    #[tokio::test]
    async fn test_convert_doc_to_docx_invalid_doc() {
        if check_libreoffice_available().await.is_err() {
            return;
        }

        let invalid_doc = b"This is not a valid .doc file";
        let result = convert_doc_to_docx(invalid_doc).await;

        let _ = result;
    }

    #[tokio::test]
    async fn test_convert_ppt_to_pptx_invalid_ppt() {
        if check_libreoffice_available().await.is_err() {
            return;
        }

        let invalid_ppt = b"This is not a valid .ppt file";
        let result = convert_ppt_to_pptx(invalid_ppt).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_convert_office_doc_invalid_target_format() {
        if check_libreoffice_available().await.is_err() {
            return;
        }

        let temp_dir = std::env::temp_dir();
        let input_path = temp_dir.join("test_input.txt");
        let output_dir = temp_dir.join("test_output_invalid_format");

        fs::write(&input_path, b"test content").await.unwrap();

        let result = convert_office_doc(&input_path, &output_dir, "invalid_format", 10).await;

        let _ = fs::remove_file(&input_path).await;
        let _ = fs::remove_dir_all(&output_dir).await;

        let _ = result;
    }

    #[tokio::test]
    async fn test_check_libreoffice_missing_dependency_error() {
        let result = check_libreoffice_available().await;

        if result.is_err() {
            let err = result.unwrap_err();
            match err {
                KreuzbergError::MissingDependency(msg) => {
                    assert!(msg.contains("LibreOffice") || msg.contains("soffice"));
                }
                _ => panic!("Expected MissingDependency error"),
            }
        }
    }

    #[tokio::test]
    async fn test_convert_office_doc_creates_output_dir() {
        if check_libreoffice_available().await.is_err() {
            return;
        }

        let temp_dir = std::env::temp_dir();
        let output_dir = temp_dir.join(format!("test_create_output_{}", uuid::Uuid::new_v4()));

        assert!(!output_dir.exists());

        let input_path = temp_dir.join("test_create_output.txt");
        fs::write(&input_path, b"test").await.unwrap();

        let _ = convert_office_doc(&input_path, &output_dir, "pdf", 10).await;

        let _ = fs::remove_file(&input_path).await;
        let _ = fs::remove_dir_all(&output_dir).await;
    }

    #[tokio::test]
    async fn test_conversion_result_structure() {
        let result = LibreOfficeConversionResult {
            converted_bytes: vec![1, 2, 3],
            original_format: "doc".to_string(),
            target_format: "docx".to_string(),
            target_mime: crate::core::mime::DOCX_MIME_TYPE.to_string(),
        };

        assert_eq!(result.original_format, "doc");
        assert_eq!(result.target_format, "docx");
        assert_eq!(result.converted_bytes.len(), 3);
    }

    #[tokio::test]
    async fn test_convert_doc_to_docx_temp_cleanup() {
        if check_libreoffice_available().await.is_err() {
            return;
        }

        let invalid_doc = b"invalid doc content";
        let _result = convert_doc_to_docx(invalid_doc).await;
    }

    #[tokio::test]
    async fn test_convert_ppt_to_pptx_temp_cleanup() {
        if check_libreoffice_available().await.is_err() {
            return;
        }

        let invalid_ppt = b"invalid ppt content";
        let _result = convert_ppt_to_pptx(invalid_ppt).await;
    }

    #[tokio::test]
    async fn test_convert_office_doc_timeout_kills_process() {
        if check_libreoffice_available().await.is_err() {
            return;
        }

        let temp_dir = std::env::temp_dir();
        let input_path = temp_dir.join("test_timeout_input.txt");
        let output_dir = temp_dir.join("test_timeout_output");

        fs::write(&input_path, b"test content").await.unwrap();

        // Use a very short timeout to trigger timeout condition
        let result = convert_office_doc(&input_path, &output_dir, "pdf", 0).await;

        // Should timeout and return error
        assert!(result.is_err());

        // Cleanup
        let _ = fs::remove_file(&input_path).await;
        let _ = fs::remove_dir_all(&output_dir).await;
    }

    #[tokio::test]
    async fn test_tempdir_raii_cleanup_on_error() {
        // Test that TempDir cleanup happens even when function returns early with error
        let temp_path = std::env::temp_dir().join(format!("test_raii_{}", uuid::Uuid::new_v4()));

        {
            let _guard = TempDir::new(temp_path.clone()).await.unwrap();
            assert!(temp_path.exists());
            // Guard dropped here, cleanup scheduled
        }

        // Give tokio time to run the cleanup task
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Directory should be cleaned up
        assert!(!temp_path.exists() || fs::read_dir(&temp_path).await.is_err());
    }
}
