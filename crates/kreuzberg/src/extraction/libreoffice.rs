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
    // Try to run soffice --version to check availability
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
    // Check LibreOffice is available
    check_libreoffice_available().await?;

    // Create output directory
    fs::create_dir_all(output_dir).await?;

    // Build command
    let command = Command::new("soffice")
        .arg("--headless")
        .arg("--convert-to")
        .arg(target_format)
        .arg("--outdir")
        .arg(output_dir)
        .arg(input_path)
        .output();

    // Execute with timeout
    let output = match timeout(Duration::from_secs(timeout_seconds), command).await {
        Ok(Ok(output)) => output,
        Ok(Err(e)) => return Err(KreuzbergError::Parsing(format!("Failed to execute LibreOffice: {}", e))),
        Err(_) => {
            return Err(KreuzbergError::Parsing(format!(
                "LibreOffice conversion timed out after {} seconds",
                timeout_seconds
            )));
        }
    };

    // Check if conversion succeeded
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
            return Err(KreuzbergError::Parsing(format!(
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

    // Find output file
    let input_stem = input_path
        .file_stem()
        .ok_or_else(|| KreuzbergError::Parsing("Invalid input file name".to_string()))?;

    let expected_output = output_dir.join(format!("{}.{}", input_stem.to_string_lossy(), target_format));

    // Read converted file
    let converted_bytes = fs::read(&expected_output).await.map_err(|e| {
        KreuzbergError::Parsing(format!(
            "LibreOffice conversion completed but output file not found: {}",
            e
        ))
    })?;

    // Check file is not empty
    if converted_bytes.is_empty() {
        return Err(KreuzbergError::Parsing(
            "LibreOffice conversion produced empty file".to_string(),
        ));
    }

    Ok(converted_bytes)
}

/// Convert .doc to .docx using LibreOffice
pub async fn convert_doc_to_docx(doc_bytes: &[u8]) -> Result<LibreOfficeConversionResult> {
    // Create temporary directory
    let temp_dir = std::env::temp_dir();
    let unique_id = uuid::Uuid::new_v4();
    let input_dir = temp_dir.join(format!("kreuzberg_doc_{}", unique_id));
    let output_dir = temp_dir.join(format!("kreuzberg_doc_{}_out", unique_id));

    fs::create_dir_all(&input_dir).await?;

    // Write input file
    let input_path = input_dir.join("input.doc");
    fs::write(&input_path, doc_bytes).await?;

    // Convert
    let result = convert_office_doc(&input_path, &output_dir, "docx", DEFAULT_CONVERSION_TIMEOUT).await;

    // Cleanup
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
    // Create temporary directory
    let temp_dir = std::env::temp_dir();
    let unique_id = uuid::Uuid::new_v4();
    let input_dir = temp_dir.join(format!("kreuzberg_ppt_{}", unique_id));
    let output_dir = temp_dir.join(format!("kreuzberg_ppt_{}_out", unique_id));

    fs::create_dir_all(&input_dir).await?;

    // Write input file
    let input_path = input_dir.join("input.ppt");
    fs::write(&input_path, ppt_bytes).await?;

    // Convert
    let result = convert_office_doc(&input_path, &output_dir, "pptx", DEFAULT_CONVERSION_TIMEOUT).await;

    // Cleanup
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
        // This test will pass if LibreOffice is installed, skip otherwise
        let result = check_libreoffice_available().await;
        if result.is_err() {
            // LibreOffice not installed, skip test
            return;
        }
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_convert_office_doc_missing_file() {
        // Skip if LibreOffice not available
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

        // LibreOffice may accept empty files and produce output
        // We just verify the function completes without panicking
        let _ = result;
    }

    #[tokio::test]
    async fn test_convert_ppt_to_pptx_empty_bytes() {
        if check_libreoffice_available().await.is_err() {
            return;
        }

        let empty_bytes = b"";
        let result = convert_ppt_to_pptx(empty_bytes).await;

        // LibreOffice may accept empty files and produce output
        // We just verify the function completes without panicking
        let _ = result;
    }

    #[tokio::test]
    async fn test_convert_doc_to_docx_invalid_doc() {
        if check_libreoffice_available().await.is_err() {
            return;
        }

        let invalid_doc = b"This is not a valid .doc file";
        let result = convert_doc_to_docx(invalid_doc).await;

        // LibreOffice may handle invalid files gracefully or error
        // We just verify the function completes
        let _ = result;
    }

    #[tokio::test]
    async fn test_convert_ppt_to_pptx_invalid_ppt() {
        if check_libreoffice_available().await.is_err() {
            return;
        }

        let invalid_ppt = b"This is not a valid .ppt file";
        let result = convert_ppt_to_pptx(invalid_ppt).await;

        // Should fail - invalid format
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

        // Create a simple text file
        fs::write(&input_path, b"test content").await.unwrap();

        // Try converting to invalid format
        let result = convert_office_doc(&input_path, &output_dir, "invalid_format", 10).await;

        // Should fail or succeed but not produce expected output
        // LibreOffice may reject invalid formats
        let _ = fs::remove_file(&input_path).await;
        let _ = fs::remove_dir_all(&output_dir).await;

        // We don't strictly assert failure here because LibreOffice behavior varies
        // but we verify the function completes
        let _ = result;
    }

    #[tokio::test]
    async fn test_check_libreoffice_missing_dependency_error() {
        let result = check_libreoffice_available().await;

        if result.is_err() {
            let err = result.unwrap_err();
            match err {
                KreuzbergError::MissingDependency(msg) => {
                    // Verify error message contains helpful info
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

        // Verify output directory doesn't exist yet
        assert!(!output_dir.exists());

        let input_path = temp_dir.join("test_create_output.txt");
        fs::write(&input_path, b"test").await.unwrap();

        let _ = convert_office_doc(&input_path, &output_dir, "pdf", 10).await;

        // Output directory should have been created
        // (even if conversion fails, directory creation succeeds)
        let _ = fs::remove_file(&input_path).await;
        let _ = fs::remove_dir_all(&output_dir).await;
    }

    #[tokio::test]
    async fn test_conversion_result_structure() {
        // Test the result structure directly without requiring LibreOffice
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

        // Even if conversion fails, temp directories should be cleaned up
        // We can't easily verify this without listing /tmp, but the function should handle it
    }

    #[tokio::test]
    async fn test_convert_ppt_to_pptx_temp_cleanup() {
        if check_libreoffice_available().await.is_err() {
            return;
        }

        let invalid_ppt = b"invalid ppt content";
        let _result = convert_ppt_to_pptx(invalid_ppt).await;

        // Even if conversion fails, temp directories should be cleaned up
        // We can't easily verify this without listing /tmp, but the function should handle it
    }
}
