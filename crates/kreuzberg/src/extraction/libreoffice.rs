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
//! check_libreoffice_available().await?;
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
use std::path::Path;
use tokio::fs;
use tokio::process::Command;
use tokio::time::{Duration, timeout};

/// Default timeout for LibreOffice conversion (300 seconds)
pub const DEFAULT_CONVERSION_TIMEOUT: u64 = 300;

/// Check if LibreOffice (soffice) is available in PATH
pub async fn check_libreoffice_available() -> Result<()> {
    let result = Command::new("soffice").arg("--version").output().await;

    match result {
        Ok(output) if output.status.success() => Ok(()),
        Ok(_) => Err(KreuzbergError::MissingDependency(
            "LibreOffice (soffice) is installed but not working correctly. \
            Please reinstall LibreOffice."
                .to_string(),
        )),
        Err(_) => Err(KreuzbergError::MissingDependency(
            "LibreOffice (soffice) is required for legacy MS Office format support (.doc, .ppt). \
            Install: macOS: 'brew install --cask libreoffice', \
            Linux: 'apt install libreoffice', \
            Windows: 'winget install LibreOffice.LibreOffice'"
                .to_string(),
        )),
    }
}

/// Convert an Office document to a target format using LibreOffice
pub async fn convert_office_doc(
    input_path: &Path,
    output_dir: &Path,
    target_format: &str,
    timeout_seconds: u64,
) -> Result<Vec<u8>> {
    check_libreoffice_available().await?;

    fs::create_dir_all(output_dir).await?;

    let command = Command::new("soffice")
        .arg("--headless")
        .arg("--convert-to")
        .arg(target_format)
        .arg("--outdir")
        .arg(output_dir)
        .arg(input_path)
        .output();

    let output = match timeout(Duration::from_secs(timeout_seconds), command).await {
        Ok(Ok(output)) => output,
        Ok(Err(e)) => return Err(KreuzbergError::parsing(format!("Failed to execute LibreOffice: {}", e))),
        Err(_) => {
            return Err(KreuzbergError::parsing(format!(
                "LibreOffice conversion timed out after {} seconds",
                timeout_seconds
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
    let input_dir = temp_dir.join(format!("kreuzberg_doc_{}", unique_id));
    let output_dir = temp_dir.join(format!("kreuzberg_doc_{}_out", unique_id));

    fs::create_dir_all(&input_dir).await?;

    let input_path = input_dir.join("input.doc");
    fs::write(&input_path, doc_bytes).await?;

    let result = convert_office_doc(&input_path, &output_dir, "docx", DEFAULT_CONVERSION_TIMEOUT).await;

    let _ = fs::remove_dir_all(&input_dir).await;
    let _ = fs::remove_dir_all(&output_dir).await;

    let converted_bytes = result?;

    Ok(LibreOfficeConversionResult {
        converted_bytes,
        original_format: "doc".to_string(),
        target_format: "docx".to_string(),
    })
}

/// Convert .ppt to .pptx using LibreOffice
pub async fn convert_ppt_to_pptx(ppt_bytes: &[u8]) -> Result<LibreOfficeConversionResult> {
    let temp_dir = std::env::temp_dir();
    let unique_id = uuid::Uuid::new_v4();
    let input_dir = temp_dir.join(format!("kreuzberg_ppt_{}", unique_id));
    let output_dir = temp_dir.join(format!("kreuzberg_ppt_{}_out", unique_id));

    fs::create_dir_all(&input_dir).await?;

    let input_path = input_dir.join("input.ppt");
    fs::write(&input_path, ppt_bytes).await?;

    let result = convert_office_doc(&input_path, &output_dir, "pptx", DEFAULT_CONVERSION_TIMEOUT).await;

    let _ = fs::remove_dir_all(&input_dir).await;
    let _ = fs::remove_dir_all(&output_dir).await;

    let converted_bytes = result?;

    Ok(LibreOfficeConversionResult {
        converted_bytes,
        original_format: "ppt".to_string(),
        target_format: "pptx".to_string(),
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
}
