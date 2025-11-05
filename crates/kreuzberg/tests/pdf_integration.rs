//! PDF integration tests that remain specific to the Rust core.
//!
//! Positive-path scenarios live in the shared fixtures that back the
//! multi-language E2E generator. This module keeps only the cases that
//! exercise Rust-specific failure handling or error propagation.

mod helpers;

use helpers::*;
use kreuzberg::core::config::ExtractionConfig;
use kreuzberg::extract_file_sync;

#[test]
fn test_pdf_password_protected_fails_gracefully() {
    if skip_if_missing("pdfs/copy_protected.pdf") {
        return;
    }

    let file_path = get_test_file_path("pdfs/copy_protected.pdf");
    let result = extract_file_sync(&file_path, None, &ExtractionConfig::default());

    match result {
        Ok(extraction_result) => {
            assert_mime_type(&extraction_result, "application/pdf");
            assert!(
                extraction_result.chunks.is_none(),
                "Chunks should be None without chunking config"
            );
            assert!(
                extraction_result.detected_languages.is_none(),
                "Language detection not enabled"
            );
        }
        Err(e) => {
            let error_msg = e.to_string().to_lowercase();
            assert!(
                error_msg.contains("password") || error_msg.contains("protected") || error_msg.contains("encrypted"),
                "Error message should indicate password/protection issue, got: {}",
                e
            );
        }
    }
}
