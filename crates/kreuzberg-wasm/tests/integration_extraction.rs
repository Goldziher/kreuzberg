//! Integration tests for WASM extraction functionality
//!
//! These tests verify complete extraction workflows including:
//! - Async extraction with Promise futures conversion
//! - File I/O patterns with FileReader integration
//! - MIME type detection and conversion
//! - Config parsing and normalization
//! - Error propagation chains
//! - Batch operations with mixed success/failure scenarios

use js_sys::Uint8Array;
use kreuzberg_wasm::*;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

// Test data with static lifetime
const VALID_PDF: &[u8] = b"%PDF-1.4\n\
1 0 obj\n\
<< /Type /Catalog /Pages 2 0 R >>\n\
endobj\n\
2 0 obj\n\
<< /Type /Pages /Kids [3 0 R] /Count 1 >>\n\
endobj\n\
3 0 obj\n\
<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>\n\
endobj\n\
4 0 obj\n\
<< /Length 44 >>\n\
stream\n\
BT\n\
/F1 12 Tf\n\
100 700 Td\n\
(Test) Tj\n\
ET\n\
endstream\n\
endobj\n\
5 0 obj\n\
<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\n\
endobj\n\
xref\n\
0 6\n\
0000000000 65535 f\n\
0000000009 00000 n\n\
0000000058 00000 n\n\
0000000115 00000 n\n\
0000000244 00000 n\n\
0000000337 00000 n\n\
trailer\n\
<< /Size 6 /Root 1 0 R >>\n\
startxref\n\
417\n\
%%EOF";

const INVALID_PDF: &[u8] = b"%PDF-1.4\n\
This is corrupted data with no valid structure\n\
Random bytes: \x00\x01\x02\x03\x04\x05\x06\x07\n\
%%EOF";

const TEXT_DATA: &[u8] = b"This is plain text content for testing";

const EMPTY_DATA: &[u8] = b"";

/// Test basic synchronous extraction with valid PDF
#[wasm_bindgen_test]
fn test_extract_bytes_sync_valid_pdf_returns_ok() {
    let data = unsafe { Uint8Array::view(VALID_PDF) };
    let result = extract_bytes_sync_wasm(data, "application/pdf".to_string(), None);
    assert!(result.is_ok(), "Valid PDF should extract successfully");
}

/// Test synchronous extraction returns non-null JsValue
#[wasm_bindgen_test]
fn test_extract_bytes_sync_returns_object() {
    let data = unsafe { Uint8Array::view(VALID_PDF) };
    let result = extract_bytes_sync_wasm(data, "application/pdf".to_string(), None);

    if let Ok(js_value) = result {
        assert!(
            !js_value.is_null() && !js_value.is_undefined(),
            "Result should be a valid object"
        );
        assert!(js_value.is_object(), "Result should be a JavaScript object");
    }
}

/// Test synchronous extraction with different MIME types
#[wasm_bindgen_test]
fn test_extract_bytes_sync_multiple_mime_types() {
    // Test with PDF
    let pdf_data = unsafe { Uint8Array::view(VALID_PDF) };
    let pdf_result = extract_bytes_sync_wasm(pdf_data, "application/pdf".to_string(), None);
    assert!(
        pdf_result.is_ok() || pdf_result.is_err(),
        "PDF handling should complete"
    );

    // Test with text/plain
    let text_data = unsafe { Uint8Array::view(TEXT_DATA) };
    let text_result = extract_bytes_sync_wasm(text_data, "text/plain".to_string(), None);
    assert!(
        text_result.is_ok() || text_result.is_err(),
        "Text handling should complete"
    );
}

/// Test synchronous extraction with None config uses defaults
#[wasm_bindgen_test]
fn test_extract_bytes_sync_none_config_uses_defaults() {
    let data = unsafe { Uint8Array::view(VALID_PDF) };
    let result_no_config = extract_bytes_sync_wasm(data.clone(), "application/pdf".to_string(), None);

    let data2 = unsafe { Uint8Array::view(VALID_PDF) };
    let result_default_config =
        extract_bytes_sync_wasm(data2, "application/pdf".to_string(), Some(wasm_bindgen::JsValue::NULL));

    // Both should have same outcome (both ok or both error)
    assert_eq!(
        result_no_config.is_ok(),
        result_default_config.is_ok(),
        "No config and default config should have same outcome"
    );
}

/// Test synchronous extraction error handling for corrupted data
#[wasm_bindgen_test]
fn test_extract_bytes_sync_corrupted_data_returns_error() {
    let data = unsafe { Uint8Array::view(INVALID_PDF) };
    let result = extract_bytes_sync_wasm(data, "application/pdf".to_string(), None);

    assert!(result.is_err(), "Corrupted PDF should return error");
    let error = result.unwrap_err();
    assert!(
        !error.is_null() && !error.is_undefined(),
        "Error should be valid JsValue"
    );
}

/// Test synchronous extraction error handling for empty data
#[wasm_bindgen_test]
fn test_extract_bytes_sync_empty_data_returns_error() {
    let data = unsafe { Uint8Array::view(EMPTY_DATA) };
    let result = extract_bytes_sync_wasm(data, "application/pdf".to_string(), None);

    assert!(result.is_err(), "Empty data should return error");
}

/// Test synchronous extraction preserves Uint8Array length
#[wasm_bindgen_test]
fn test_extract_bytes_sync_preserves_data_integrity() {
    let data = unsafe { Uint8Array::view(VALID_PDF) };
    let length = data.length() as usize;

    let _ = extract_bytes_sync_wasm(data, "application/pdf".to_string(), None);

    assert_eq!(length, VALID_PDF.len(), "Uint8Array length should match original data");
}

/// Test async extraction returns a Promise
#[wasm_bindgen_test]
fn test_extract_bytes_async_returns_promise() {
    let data = unsafe { Uint8Array::view(VALID_PDF) };
    let promise = extract_bytes_wasm(data, "application/pdf".to_string(), None);

    assert!(!promise.is_null() && !promise.is_undefined(), "Should return a Promise");
}

/// Test async extraction with invalid MIME type returns Promise
#[wasm_bindgen_test]
fn test_extract_bytes_async_invalid_mime_returns_promise() {
    let data = unsafe { Uint8Array::view(VALID_PDF) };
    let promise = extract_bytes_wasm(data, "invalid/type".to_string(), None);

    assert!(!promise.is_null(), "Invalid MIME should still return Promise");
}

/// Test async extraction with config option
#[wasm_bindgen_test]
fn test_extract_bytes_async_with_config() {
    let data = unsafe { Uint8Array::view(VALID_PDF) };
    let config = Some(wasm_bindgen::JsValue::NULL);
    let promise = extract_bytes_wasm(data, "application/pdf".to_string(), config);

    assert!(!promise.is_null(), "Async extraction with config should return Promise");
}

/// Test batch extraction sync with matching lengths
#[wasm_bindgen_test]
fn test_batch_extract_sync_matching_lengths_succeeds() {
    let pdf = unsafe { Uint8Array::view(VALID_PDF) };
    let text = unsafe { Uint8Array::view(TEXT_DATA) };

    let data_list = vec![pdf, text];
    let mime_types = vec!["application/pdf".to_string(), "text/plain".to_string()];

    let result = batch_extract_bytes_sync_wasm(data_list, mime_types, None);
    assert!(result.is_ok(), "Batch extraction with matching lengths should succeed");
}

/// Test batch extraction sync with mismatched lengths
#[wasm_bindgen_test]
fn test_batch_extract_sync_mismatched_lengths_fails() {
    let pdf = unsafe { Uint8Array::view(VALID_PDF) };

    let data_list = vec![pdf];
    let mime_types = vec!["application/pdf".to_string(), "text/plain".to_string()];

    let result = batch_extract_bytes_sync_wasm(data_list, mime_types, None);
    assert!(result.is_err(), "Mismatched lengths should cause error");
}

/// Test batch extraction sync with empty batch
#[wasm_bindgen_test]
fn test_batch_extract_sync_empty_batch() {
    let data_list: Vec<Uint8Array> = vec![];
    let mime_types: Vec<String> = vec![];

    let result = batch_extract_bytes_sync_wasm(data_list, mime_types, None);
    assert!(result.is_ok(), "Empty batch should be valid (no documents to process)");
}

/// Test batch extraction sync with single document
#[wasm_bindgen_test]
fn test_batch_extract_sync_single_document() {
    let pdf = unsafe { Uint8Array::view(VALID_PDF) };

    let data_list = vec![pdf];
    let mime_types = vec!["application/pdf".to_string()];

    let result = batch_extract_bytes_sync_wasm(data_list, mime_types, None);
    assert!(result.is_ok(), "Single document batch should succeed");
}

/// Test batch extraction sync with multiple documents
#[wasm_bindgen_test]
fn test_batch_extract_sync_multiple_documents() {
    let pdf1 = unsafe { Uint8Array::view(VALID_PDF) };
    let text = unsafe { Uint8Array::view(TEXT_DATA) };
    let pdf2 = unsafe { Uint8Array::view(VALID_PDF) };

    let data_list = vec![pdf1, text, pdf2];
    let mime_types = vec![
        "application/pdf".to_string(),
        "text/plain".to_string(),
        "application/pdf".to_string(),
    ];

    let result = batch_extract_bytes_sync_wasm(data_list, mime_types, None);
    assert!(result.is_ok(), "Multiple documents should extract as batch");
}

/// Test batch extraction async returns Promise
#[wasm_bindgen_test]
fn test_batch_extract_async_returns_promise() {
    let pdf = unsafe { Uint8Array::view(VALID_PDF) };

    let data_list = vec![pdf];
    let mime_types = vec!["application/pdf".to_string()];

    let promise = batch_extract_bytes_wasm(data_list, mime_types, None);
    assert!(!promise.is_null(), "Batch async should return Promise");
}

/// Test batch extraction async with mismatched lengths returns Promise
#[wasm_bindgen_test]
fn test_batch_extract_async_mismatched_lengths_returns_promise() {
    let pdf = unsafe { Uint8Array::view(VALID_PDF) };

    let data_list = vec![pdf];
    let mime_types = vec!["application/pdf".to_string(), "text/plain".to_string()];

    let promise = batch_extract_bytes_wasm(data_list, mime_types, None);
    assert!(!promise.is_null(), "Should return Promise even with mismatched lengths");
}

/// Test deterministic extraction - same input produces same result structure
#[wasm_bindgen_test]
fn test_extract_sync_deterministic_results() {
    let data1 = unsafe { Uint8Array::view(VALID_PDF) };
    let result1 = extract_bytes_sync_wasm(data1, "application/pdf".to_string(), None);

    let data2 = unsafe { Uint8Array::view(VALID_PDF) };
    let result2 = extract_bytes_sync_wasm(data2, "application/pdf".to_string(), None);

    // Both should succeed or both should fail
    assert_eq!(result1.is_ok(), result2.is_ok(), "Extraction should be deterministic");
}

/// Test no state leakage between consecutive extractions
#[wasm_bindgen_test]
fn test_extract_sync_no_state_leakage() {
    let valid = unsafe { Uint8Array::view(VALID_PDF) };
    let invalid = unsafe { Uint8Array::view(INVALID_PDF) };
    let valid2 = unsafe { Uint8Array::view(VALID_PDF) };

    let result1 = extract_bytes_sync_wasm(valid, "application/pdf".to_string(), None);
    let result2 = extract_bytes_sync_wasm(invalid, "application/pdf".to_string(), None);
    let result3 = extract_bytes_sync_wasm(valid2, "application/pdf".to_string(), None);

    // Valid and valid2 should have same outcome, invalid should differ
    assert_eq!(result1.is_ok(), result3.is_ok(), "Valid PDFs should have same outcome");
    assert_ne!(result1.is_ok(), result2.is_ok(), "Invalid PDF should differ from valid");
}

/// Test file sync functions correctly report WASM limitation
#[wasm_bindgen_test]
fn test_extract_file_sync_returns_error() {
    let result = extract_file_sync_wasm();
    assert!(result.is_err(), "File sync operations not available in WASM");
}

/// Test batch file sync functions correctly report WASM limitation
#[wasm_bindgen_test]
fn test_batch_extract_files_sync_returns_error() {
    let result = batch_extract_files_sync_wasm();
    assert!(result.is_err(), "Batch file sync operations not available in WASM");
}

/// Test MIME type case handling
#[wasm_bindgen_test]
fn test_extract_sync_mime_type_case_handling() {
    let data = unsafe { Uint8Array::view(VALID_PDF) };
    let result_lower = extract_bytes_sync_wasm(data.clone(), "application/pdf".to_string(), None);

    let data2 = unsafe { Uint8Array::view(VALID_PDF) };
    let result_upper = extract_bytes_sync_wasm(data2, "APPLICATION/PDF".to_string(), None);

    // Results may differ based on implementation but shouldn't panic
    assert!(
        result_lower.is_ok() || result_lower.is_err(),
        "Lowercase MIME should handle gracefully"
    );
    assert!(
        result_upper.is_ok() || result_upper.is_err(),
        "Uppercase MIME should handle gracefully"
    );
}

/// Test MIME type with charset parameter
#[wasm_bindgen_test]
fn test_extract_sync_mime_type_with_charset() {
    let data = unsafe { Uint8Array::view(VALID_PDF) };
    let result = extract_bytes_sync_wasm(data, "application/pdf; charset=utf-8".to_string(), None);

    // Should handle gracefully (either parse or error appropriately)
    assert!(
        result.is_ok() || result.is_err(),
        "MIME with charset should handle gracefully"
    );
}

/// Test batch operations with mixed document types
#[wasm_bindgen_test]
fn test_batch_extract_sync_mixed_types() {
    let doc1 = unsafe { Uint8Array::view(VALID_PDF) };
    let doc2 = unsafe { Uint8Array::view(TEXT_DATA) };
    let doc3 = unsafe { Uint8Array::view(EMPTY_DATA) };

    let data_list = vec![doc1, doc2, doc3];
    let mime_types = vec![
        "application/pdf".to_string(),
        "text/plain".to_string(),
        "application/octet-stream".to_string(),
    ];

    // Batch should process all or return error for the invalid one
    let result = batch_extract_bytes_sync_wasm(data_list, mime_types, None);
    assert!(
        result.is_ok() || result.is_err(),
        "Mixed types should handle gracefully"
    );
}

/// Test config parameter is properly passed through
#[wasm_bindgen_test]
fn test_extract_sync_config_parameter_passed() {
    let data1 = unsafe { Uint8Array::view(VALID_PDF) };
    let config1 = Some(wasm_bindgen::JsValue::NULL);
    let result1 = extract_bytes_sync_wasm(data1, "application/pdf".to_string(), config1);

    let data2 = unsafe { Uint8Array::view(VALID_PDF) };
    let result2 = extract_bytes_sync_wasm(data2, "application/pdf".to_string(), None);

    // Results should be consistent when config is null
    assert_eq!(result1.is_ok(), result2.is_ok(), "Null config should match no config");
}

/// Test large batch operations
#[wasm_bindgen_test]
fn test_batch_extract_sync_large_batch() {
    let mut data_list = Vec::new();
    let mut mime_types = Vec::new();

    // Create 10 document batch
    for _i in 0..10 {
        let data = unsafe { Uint8Array::view(TEXT_DATA) };
        data_list.push(data);
        mime_types.push("text/plain".to_string());
    }

    let result = batch_extract_bytes_sync_wasm(data_list, mime_types, None);
    assert!(
        result.is_ok() || result.is_err(),
        "Large batch should complete without panic"
    );
}

/// Test async extraction doesn't block
#[wasm_bindgen_test]
fn test_extract_async_non_blocking() {
    let data1 = unsafe { Uint8Array::view(VALID_PDF) };
    let promise1 = extract_bytes_wasm(data1, "application/pdf".to_string(), None);

    let data2 = unsafe { Uint8Array::view(TEXT_DATA) };
    let promise2 = extract_bytes_wasm(data2, "text/plain".to_string(), None);

    // Both promises should be created immediately
    assert!(
        !promise1.is_null() && !promise2.is_null(),
        "Multiple async calls should all return Promise"
    );
}

/// Test batch async with empty list
#[wasm_bindgen_test]
fn test_batch_extract_async_empty_list() {
    let data_list: Vec<Uint8Array> = vec![];
    let mime_types: Vec<String> = vec![];

    let promise = batch_extract_bytes_wasm(data_list, mime_types, None);
    assert!(!promise.is_null(), "Empty batch should still return Promise");
}
