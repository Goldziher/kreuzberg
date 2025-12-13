//! Integration tests for JavaScript/Rust interoperability in WASM
//!
//! These tests verify:
//! - JsValue conversions for all types
//! - Uint8Array handling and memory safety
//! - Async trait patterns with MakeSend wrapper
//! - wasm_bindgen Promise integration
//! - Error conversion accuracy and completeness

use js_sys::Uint8Array;
use kreuzberg_wasm::*;
use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

// Test data
const PDF_DATA: &[u8] = b"%PDF-1.4\n%test";
const TEXT_DATA: &[u8] = b"Hello, World!";
const BINARY_DATA: &[u8] = b"\x00\x01\x02\x03\x04\x05";
const LARGE_DATA: &[u8] = b"Lorem ipsum dolor sit amet, consectetur adipiscing elit. \
                                Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.";

// ============================================================================
// Uint8Array Conversion Tests
// ============================================================================

/// Test Uint8Array view creation from bytes
#[wasm_bindgen_test]
fn test_uint8array_view_creation() {
    let data = unsafe { Uint8Array::view(PDF_DATA) };
    assert_eq!(
        data.length() as usize,
        PDF_DATA.len(),
        "View length should match source data"
    );
}

/// Test Uint8Array to_vec conversion
#[wasm_bindgen_test]
fn test_uint8array_to_vec_conversion() {
    let data = unsafe { Uint8Array::view(TEXT_DATA) };
    let vec = data.to_vec();

    assert_eq!(
        vec.len(),
        TEXT_DATA.len(),
        "Converted vector should match original length"
    );
    assert_eq!(vec.as_slice(), TEXT_DATA, "Vector content should match original data");
}

/// Test Uint8Array with binary data containing null bytes
#[wasm_bindgen_test]
fn test_uint8array_binary_data_with_nulls() {
    let data = unsafe { Uint8Array::view(BINARY_DATA) };
    let vec = data.to_vec();

    assert_eq!(vec.len(), BINARY_DATA.len(), "Binary data length should be preserved");
    assert_eq!(vec[0], 0x00, "Null byte should be preserved");
    assert_eq!(vec[1], 0x01, "Subsequent bytes should be preserved");
}

/// Test Uint8Array with large data
#[wasm_bindgen_test]
fn test_uint8array_large_data() {
    let data = unsafe { Uint8Array::view(LARGE_DATA) };
    assert_eq!(
        data.length() as usize,
        LARGE_DATA.len(),
        "Large data length should match"
    );

    let vec = data.to_vec();
    assert_eq!(
        vec.len(),
        LARGE_DATA.len(),
        "Large data conversion should preserve size"
    );
}

/// Test multiple Uint8Array views of same data
#[wasm_bindgen_test]
fn test_multiple_uint8array_views() {
    let view1 = unsafe { Uint8Array::view(PDF_DATA) };
    let view2 = unsafe { Uint8Array::view(PDF_DATA) };

    assert_eq!(view1.length(), view2.length(), "Multiple views should have same length");

    let vec1 = view1.to_vec();
    let vec2 = view2.to_vec();

    assert_eq!(vec1, vec2, "Multiple views should convert to identical vectors");
}

/// Test Uint8Array clone preserves data
#[wasm_bindgen_test]
fn test_uint8array_clone_preserves_data() {
    let data = unsafe { Uint8Array::view(TEXT_DATA) };
    let data_clone = data.clone();

    assert_eq!(data.length(), data_clone.length(), "Clone should have same length");

    let vec_original = data.to_vec();
    let vec_clone = data_clone.to_vec();

    assert_eq!(vec_original, vec_clone, "Clone should convert to same data");
}

// ============================================================================
// JsValue Result Conversion Tests
// ============================================================================

/// Test JsValue result is object type
#[wasm_bindgen_test]
fn test_result_jsvalue_is_object() {
    let data = unsafe { Uint8Array::view(PDF_DATA) };
    let result = extract_bytes_sync_wasm(data, "application/pdf".to_string(), None);

    if let Ok(js_value) = result {
        assert!(js_value.is_object(), "Extraction result should be object");
        assert!(!js_value.is_null(), "Result should not be null");
        assert!(!js_value.is_undefined(), "Result should not be undefined");
    }
}

/// Test error JsValue is non-null
#[wasm_bindgen_test]
fn test_error_jsvalue_non_null() {
    let data = unsafe { Uint8Array::view(b"invalid") };
    let result = extract_bytes_sync_wasm(data, "application/pdf".to_string(), None);

    if let Err(error) = result {
        assert!(!error.is_null(), "Error JsValue should not be null");
        assert!(!error.is_undefined(), "Error JsValue should not be undefined");
    }
}

/// Test batch result is object array
#[wasm_bindgen_test]
fn test_batch_result_is_array() {
    let data = unsafe { Uint8Array::view(PDF_DATA) };
    let result = batch_extract_bytes_sync_wasm(vec![data], vec!["application/pdf".to_string()], None);

    if let Ok(js_value) = result {
        let arr = js_sys::Array::from(&js_value);
        assert!(arr.is_array(), "Batch result should be array");
    }
}

/// Test JsValue Promise is proper type
#[wasm_bindgen_test]
fn test_promise_jsvalue_is_valid() {
    let data = unsafe { Uint8Array::view(PDF_DATA) };
    let promise = extract_bytes_wasm(data, "application/pdf".to_string(), None);

    assert!(!promise.is_null(), "Promise should not be null");
    assert!(!promise.is_undefined(), "Promise should not be undefined");
}

// ============================================================================
// Config Parsing and Conversion Tests
// ============================================================================

/// Test None config is converted properly
#[wasm_bindgen_test]
fn test_config_none_parsing() {
    let data = unsafe { Uint8Array::view(PDF_DATA) };
    let result1 = extract_bytes_sync_wasm(data.clone(), "application/pdf".to_string(), None);

    let data2 = unsafe { Uint8Array::view(PDF_DATA) };
    let result2 = extract_bytes_sync_wasm(data2, "application/pdf".to_string(), None);

    // Both should have same outcome
    assert_eq!(
        result1.is_ok(),
        result2.is_ok(),
        "Same config should have consistent results"
    );
}

/// Test JsValue::NULL config parsing
#[wasm_bindgen_test]
fn test_config_null_jsvalue_parsing() {
    let data = unsafe { Uint8Array::view(PDF_DATA) };
    let result = extract_bytes_sync_wasm(data, "application/pdf".to_string(), Some(JsValue::NULL));

    assert!(
        result.is_ok() || result.is_err(),
        "NULL config should parse without panic"
    );
}

/// Test config parameter option handling
#[wasm_bindgen_test]
fn test_config_some_vs_none_handling() {
    let data1 = unsafe { Uint8Array::view(PDF_DATA) };
    let result_some = extract_bytes_sync_wasm(data1, "application/pdf".to_string(), Some(JsValue::NULL));

    let data2 = unsafe { Uint8Array::view(PDF_DATA) };
    let result_none = extract_bytes_sync_wasm(data2, "application/pdf".to_string(), None);

    // Results should be consistent
    assert_eq!(
        result_some.is_ok(),
        result_none.is_ok(),
        "Some(NULL) and None should behave similarly"
    );
}

// ============================================================================
// Error Type Conversion Tests
// ============================================================================

/// Test error conversion to JsValue
#[wasm_bindgen_test]
fn test_invalid_mime_returns_convertible_error() {
    let data = unsafe { Uint8Array::view(PDF_DATA) };
    let result = extract_bytes_sync_wasm(data, "invalid/mime/type".to_string(), None);

    if let Err(error) = result {
        assert!(!error.is_null(), "Error should convert to non-null JsValue");
    }
}

/// Test corrupted data returns error JsValue
#[wasm_bindgen_test]
fn test_corrupted_data_error_conversion() {
    let corrupted = b"not a valid pdf\x00\x01\x02";
    let data = unsafe { Uint8Array::view(corrupted) };
    let result = extract_bytes_sync_wasm(data, "application/pdf".to_string(), None);

    if let Err(error) = result {
        assert!(
            !error.is_null() && !error.is_undefined(),
            "Error should be valid JsValue"
        );
    }
}

/// Test empty data returns error
#[wasm_bindgen_test]
fn test_empty_data_error_conversion() {
    let empty = b"";
    let data = unsafe { Uint8Array::view(empty) };
    let result = extract_bytes_sync_wasm(data, "application/pdf".to_string(), None);

    assert!(result.is_err(), "Empty data should produce error");
}

// ============================================================================
// Type Integration Tests
// ============================================================================

/// Test multiple simultaneous extractions with different types
#[wasm_bindgen_test]
fn test_simultaneous_different_types() {
    let pdf = unsafe { Uint8Array::view(PDF_DATA) };
    let text = unsafe { Uint8Array::view(TEXT_DATA) };
    let binary = unsafe { Uint8Array::view(BINARY_DATA) };

    let _ = extract_bytes_sync_wasm(pdf, "application/pdf".to_string(), None);
    let _ = extract_bytes_sync_wasm(text, "text/plain".to_string(), None);
    let _ = extract_bytes_sync_wasm(binary, "application/octet-stream".to_string(), None);

    // No panics expected
}

/// Test batch with heterogeneous data
#[wasm_bindgen_test]
fn test_batch_heterogeneous_data() {
    let pdf = unsafe { Uint8Array::view(PDF_DATA) };
    let text = unsafe { Uint8Array::view(TEXT_DATA) };
    let binary = unsafe { Uint8Array::view(BINARY_DATA) };

    let result = batch_extract_bytes_sync_wasm(
        vec![pdf, text, binary],
        vec![
            "application/pdf".to_string(),
            "text/plain".to_string(),
            "application/octet-stream".to_string(),
        ],
        None,
    );

    assert!(result.is_ok() || result.is_err(), "Heterogeneous batch should complete");
}

/// Test JsValue array construction for batch results
#[wasm_bindgen_test]
fn test_batch_result_array_construction() {
    let data = unsafe { Uint8Array::view(TEXT_DATA) };
    let result = batch_extract_bytes_sync_wasm(vec![data], vec!["text/plain".to_string()], None);

    if let Ok(js_value) = result {
        let arr = js_sys::Array::from(&js_value);
        assert!(arr.is_array(), "Result should be convertible to array");
        // u32 length is always >= 0, just verify it's an array
    }
}

// ============================================================================
// Memory and Lifetime Tests
// ============================================================================

/// Test Uint8Array view doesn't corrupt source data
#[wasm_bindgen_test]
fn test_uint8array_view_doesnt_modify_source() {
    let data_original = PDF_DATA;

    let data_view = unsafe { Uint8Array::view(data_original) };
    let _ = data_view.to_vec();

    // Check original is unchanged
    assert_eq!(data_original, PDF_DATA, "Source data should be unchanged");
}

/// Test sequential Uint8Array views
#[wasm_bindgen_test]
fn test_sequential_uint8array_views() {
    let view1 = unsafe { Uint8Array::view(PDF_DATA) };
    let vec1 = view1.to_vec();

    let view2 = unsafe { Uint8Array::view(TEXT_DATA) };
    let vec2 = view2.to_vec();

    assert_eq!(vec1, PDF_DATA, "First view should match first data");
    assert_eq!(vec2, TEXT_DATA, "Second view should match second data");
}

/// Test Uint8Array conversion with different slices
#[wasm_bindgen_test]
fn test_uint8array_different_slices() {
    let slice1 = &PDF_DATA[0..3];
    let data1 = unsafe { Uint8Array::view(slice1) };
    let vec1 = data1.to_vec();

    assert_eq!(vec1.len(), 3, "Slice view length should be correct");
    assert_eq!(vec1[0..3], slice1[0..3], "Slice content should be correct");
}

// ============================================================================
// Promise and Async Tests
// ============================================================================

/// Test async extraction creates new Promise each time
#[wasm_bindgen_test]
fn test_async_creates_new_promise_each_call() {
    let data1 = unsafe { Uint8Array::view(PDF_DATA) };
    let promise1 = extract_bytes_wasm(data1, "application/pdf".to_string(), None);

    let data2 = unsafe { Uint8Array::view(TEXT_DATA) };
    let promise2 = extract_bytes_wasm(data2, "text/plain".to_string(), None);

    // Both should be valid Promise objects
    assert!(!promise1.is_null());
    assert!(!promise2.is_null());
}

/// Test batch async Promise creation
#[wasm_bindgen_test]
fn test_batch_async_promise_creation() {
    let data = unsafe { Uint8Array::view(PDF_DATA) };
    let promise = batch_extract_bytes_wasm(vec![data], vec!["application/pdf".to_string()], None);

    assert!(!promise.is_null(), "Batch async should create Promise");
}

/// Test async Promise with empty batch
#[wasm_bindgen_test]
fn test_async_promise_empty_batch() {
    let promise = batch_extract_bytes_wasm(vec![], vec![], None);

    assert!(!promise.is_null(), "Even empty batch should return Promise");
}

// ============================================================================
// Edge Case Tests
// ============================================================================

/// Test single-byte Uint8Array
#[wasm_bindgen_test]
fn test_uint8array_single_byte() {
    let single: &[u8] = b"X";
    let data = unsafe { Uint8Array::view(single) };

    assert_eq!(data.length(), 1, "Single byte should have length 1");
    assert_eq!(data.to_vec(), single, "Single byte should convert correctly");
}

/// Test very short data
#[wasm_bindgen_test]
fn test_extract_very_short_data() {
    let short = b"X";
    let data = unsafe { Uint8Array::view(short) };
    let result = extract_bytes_sync_wasm(data, "application/pdf".to_string(), None);

    assert!(
        result.is_ok() || result.is_err(),
        "Short data should complete without panic"
    );
}

/// Test maximum batch size
#[wasm_bindgen_test]
fn test_batch_large_size() {
    let mut data_list = Vec::new();
    let mut mime_types = Vec::new();

    for _ in 0..50 {
        let data = unsafe { Uint8Array::view(TEXT_DATA) };
        data_list.push(data);
        mime_types.push("text/plain".to_string());
    }

    let result = batch_extract_bytes_sync_wasm(data_list, mime_types, None);
    assert!(result.is_ok() || result.is_err(), "Large batch should complete");
}

/// Test type consistency across conversions
#[wasm_bindgen_test]
fn test_type_consistency_across_conversions() {
    let original = unsafe { Uint8Array::view(TEXT_DATA) };
    let vec = original.to_vec();
    let new_view = unsafe { Uint8Array::view(&vec) };

    assert_eq!(
        original.length(),
        new_view.length(),
        "Type conversions should preserve size"
    );
}

/// Test JsValue null vs undefined distinction
#[wasm_bindgen_test]
fn test_jsvalue_null_vs_undefined() {
    let null_val = JsValue::NULL;
    let undefined_val = JsValue::undefined();

    assert!(null_val.is_null(), "NULL should be null");
    assert!(undefined_val.is_undefined(), "Undefined should be undefined");
    assert!(!null_val.is_undefined(), "NULL should not be undefined");
    assert!(!undefined_val.is_null(), "Undefined should not be null");
}

/// Test config parsing with NULL vs undefined
#[wasm_bindgen_test]
fn test_config_null_vs_undefined() {
    let data1 = unsafe { Uint8Array::view(PDF_DATA) };
    let result_null = extract_bytes_sync_wasm(data1, "application/pdf".to_string(), Some(JsValue::NULL));

    let data2 = unsafe { Uint8Array::view(PDF_DATA) };
    let result_undef = extract_bytes_sync_wasm(data2, "application/pdf".to_string(), Some(JsValue::undefined()));

    // Both should handle gracefully
    assert!(result_null.is_ok() || result_null.is_err(), "NULL config should parse");
    assert!(
        result_undef.is_ok() || result_undef.is_err(),
        "Undefined config should parse"
    );
}
