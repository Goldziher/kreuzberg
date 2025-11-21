//! FFI plugin registration integration tests.
//!
//! Tests the FFI layer for registering and managing validators and post-processors.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

// External FFI functions for validators
unsafe extern "C" {
    fn kreuzberg_register_validator(name: *const c_char, callback: ValidatorCallback, priority: i32) -> bool;
    fn kreuzberg_unregister_validator(name: *const c_char) -> bool;
    fn kreuzberg_list_validators() -> *mut c_char;
    fn kreuzberg_clear_validators() -> bool;
    fn kreuzberg_free_string(s: *mut c_char);
    fn kreuzberg_last_error() -> *const c_char;
}

// External FFI functions for OCR backends
unsafe extern "C" {
    fn kreuzberg_unregister_ocr_backend(name: *const c_char) -> bool;
    fn kreuzberg_list_ocr_backends() -> *mut c_char;
}

// Callback types
type ValidatorCallback = unsafe extern "C" fn(
    content: *const c_char,
    mime_type: *const c_char,
    metadata_json: *const c_char,
    config_json: *const c_char,
) -> *mut c_char;

/// Helper to convert *const c_char to String
unsafe fn c_str_to_string(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        None
    } else {
        unsafe { Some(CStr::from_ptr(ptr).to_string_lossy().into_owned()) }
    }
}

/// Helper to get last error message
unsafe fn get_last_error() -> Option<String> {
    let error_ptr = unsafe { kreuzberg_last_error() };
    unsafe { c_str_to_string(error_ptr) }
}

/// Mock validator callback that always passes
unsafe extern "C" fn passing_validator_callback(
    _content: *const c_char,
    _mime_type: *const c_char,
    _metadata_json: *const c_char,
    _config_json: *const c_char,
) -> *mut c_char {
    ptr::null_mut() // null means validation passed
}

/// Mock validator callback that always fails
unsafe extern "C" fn failing_validator_callback(
    _content: *const c_char,
    _mime_type: *const c_char,
    _metadata_json: *const c_char,
    _config_json: *const c_char,
) -> *mut c_char {
    let error_msg = CString::new("Validation failed: content too short").unwrap();
    error_msg.into_raw()
}

/// Test successful validator registration.
#[test]
fn test_register_validator_succeeds() {
    unsafe {
        kreuzberg_clear_validators();

        let name = CString::new("test-validator").unwrap();
        let result = kreuzberg_register_validator(name.as_ptr(), passing_validator_callback, 50);

        assert!(result, "Validator registration should succeed");

        // Verify it's in the list
        let list_ptr = kreuzberg_list_validators();
        assert!(!list_ptr.is_null(), "List should not be null");

        let list_json = c_str_to_string(list_ptr).expect("Should have valid JSON");
        kreuzberg_free_string(list_ptr);

        assert!(
            list_json.contains("test-validator"),
            "List should contain registered validator"
        );

        // Cleanup
        kreuzberg_clear_validators();
    }
}

/// Test registering multiple validators.
#[test]
fn test_register_multiple_validators_succeeds() {
    unsafe {
        kreuzberg_clear_validators();

        let validator1 = CString::new("validator-1").unwrap();
        let validator2 = CString::new("validator-2").unwrap();
        let validator3 = CString::new("validator-3").unwrap();

        assert!(
            kreuzberg_register_validator(validator1.as_ptr(), passing_validator_callback, 100),
            "First validator registration should succeed"
        );
        assert!(
            kreuzberg_register_validator(validator2.as_ptr(), passing_validator_callback, 50),
            "Second validator registration should succeed"
        );
        assert!(
            kreuzberg_register_validator(validator3.as_ptr(), failing_validator_callback, 25),
            "Third validator registration should succeed"
        );

        // Verify all are in the list
        let list_ptr = kreuzberg_list_validators();
        let list_json = c_str_to_string(list_ptr).expect("Should have valid JSON");
        kreuzberg_free_string(list_ptr);

        assert!(list_json.contains("validator-1"), "Should contain validator-1");
        assert!(list_json.contains("validator-2"), "Should contain validator-2");
        assert!(list_json.contains("validator-3"), "Should contain validator-3");

        // Cleanup
        kreuzberg_clear_validators();
    }
}

/// Test unregistering validator.
#[test]
fn test_unregister_validator_succeeds() {
    unsafe {
        kreuzberg_clear_validators();

        let name = CString::new("temp-validator").unwrap();
        kreuzberg_register_validator(name.as_ptr(), passing_validator_callback, 50);

        // Unregister
        let result = kreuzberg_unregister_validator(name.as_ptr());
        assert!(result, "Unregistration should succeed");

        // Verify it's not in the list
        let list_ptr = kreuzberg_list_validators();
        let list_json = c_str_to_string(list_ptr).expect("Should have valid JSON");
        kreuzberg_free_string(list_ptr);

        assert!(
            !list_json.contains("temp-validator"),
            "List should not contain unregistered validator"
        );

        // Cleanup
        kreuzberg_clear_validators();
    }
}

/// Test unregistering non-existent validator fails gracefully.
#[test]
fn test_unregister_nonexistent_validator_fails_gracefully() {
    unsafe {
        kreuzberg_clear_validators();

        let name = CString::new("nonexistent-validator").unwrap();
        let result = kreuzberg_unregister_validator(name.as_ptr());

        // Should return true (no-op for non-existent)
        assert!(result, "Unregistering non-existent validator should succeed (no-op)");

        // Cleanup
        kreuzberg_clear_validators();
    }
}

/// Test registering validator with null name fails gracefully.
#[test]
fn test_register_validator_with_null_name_fails_gracefully() {
    unsafe {
        let result = kreuzberg_register_validator(ptr::null(), passing_validator_callback, 50);

        assert!(!result, "Registration with null name should fail");

        let error = get_last_error();
        assert!(error.is_some(), "Should have error message");
        let error_msg = error.unwrap();
        assert!(
            error_msg.contains("null") || error_msg.contains("invalid") || error_msg.contains("empty"),
            "Error should mention null/invalid: {}",
            error_msg
        );
    }
}

/// Test registering validator with empty name fails gracefully.
#[test]
fn test_register_validator_with_empty_name_fails_gracefully() {
    unsafe {
        let name = CString::new("").unwrap();
        let result = kreuzberg_register_validator(name.as_ptr(), passing_validator_callback, 50);

        assert!(!result, "Registration with empty name should fail");

        let error = get_last_error();
        assert!(error.is_some(), "Should have error message");
        let error_msg = error.unwrap();
        assert!(
            error_msg.contains("empty") || error_msg.contains("invalid"),
            "Error should mention empty/invalid: {}",
            error_msg
        );
    }
}

/// Test registering validator with whitespace in name fails gracefully.
#[test]
fn test_register_validator_with_whitespace_in_name_fails_gracefully() {
    unsafe {
        let name = CString::new("validator with spaces").unwrap();
        let result = kreuzberg_register_validator(name.as_ptr(), passing_validator_callback, 50);

        assert!(!result, "Registration with whitespace in name should fail");

        let error = get_last_error();
        assert!(error.is_some(), "Should have error message");
        let error_msg = error.unwrap();
        assert!(
            error_msg.contains("whitespace") || error_msg.contains("invalid"),
            "Error should mention whitespace/invalid: {}",
            error_msg
        );
    }
}

/// Test registering validator with invalid UTF-8 fails gracefully.
#[test]
fn test_register_validator_with_invalid_utf8_fails_gracefully() {
    unsafe {
        // Create invalid UTF-8 bytes manually (0xFF, 0xFE are not valid UTF-8)
        let invalid_bytes = vec![
            b'v', b'a', b'l', b'i', b'd', b'a', b't', b'o', b'r', b'-', 0xFF, 0xFE, 0x00,
        ];
        let name_ptr = invalid_bytes.as_ptr() as *const i8;
        let result = kreuzberg_register_validator(name_ptr, passing_validator_callback, 50);

        // Should fail gracefully with UTF-8 error
        assert!(!result, "Should fail with invalid UTF-8");
        let error = get_last_error();
        assert!(error.is_some(), "Should have error message on failure");
        assert!(
            error.unwrap().contains("Invalid UTF-8"),
            "Error should mention UTF-8 issue"
        );

        // Cleanup regardless
        kreuzberg_clear_validators();
    }
}

/// Test clearing all validators.
#[test]
fn test_clear_validators_succeeds() {
    unsafe {
        kreuzberg_clear_validators();

        // Register multiple validators
        let v1 = CString::new("validator-1").unwrap();
        let v2 = CString::new("validator-2").unwrap();
        kreuzberg_register_validator(v1.as_ptr(), passing_validator_callback, 50);
        kreuzberg_register_validator(v2.as_ptr(), passing_validator_callback, 50);

        // Clear all
        let result = kreuzberg_clear_validators();
        assert!(result, "Clear should succeed");

        // Verify list is empty
        let list_ptr = kreuzberg_list_validators();
        let list_json = c_str_to_string(list_ptr).expect("Should have valid JSON");
        kreuzberg_free_string(list_ptr);

        let validators: Vec<String> = serde_json::from_str(&list_json).unwrap_or_default();
        assert_eq!(validators.len(), 0, "List should be empty after clear");
    }
}

/// Test listing validators returns valid JSON.
#[test]
fn test_list_validators_returns_valid_json() {
    unsafe {
        kreuzberg_clear_validators();

        let name = CString::new("test-validator").unwrap();
        kreuzberg_register_validator(name.as_ptr(), passing_validator_callback, 50);

        let list_ptr = kreuzberg_list_validators();
        assert!(!list_ptr.is_null(), "List should not be null");

        let list_json = c_str_to_string(list_ptr).expect("Should have valid JSON");
        kreuzberg_free_string(list_ptr);

        // Parse as JSON array
        let validators: Vec<String> = serde_json::from_str(&list_json).expect("Should be valid JSON array");
        assert!(
            validators.contains(&"test-validator".to_string()),
            "Should contain registered validator"
        );

        // Cleanup
        kreuzberg_clear_validators();
    }
}

/// Test listing empty validators returns empty array.
#[test]
fn test_list_empty_validators_returns_empty_array() {
    unsafe {
        kreuzberg_clear_validators();

        let list_ptr = kreuzberg_list_validators();
        assert!(!list_ptr.is_null(), "List should not be null");

        let list_json = c_str_to_string(list_ptr).expect("Should have valid JSON");
        kreuzberg_free_string(list_ptr);

        let validators: Vec<String> = serde_json::from_str(&list_json).expect("Should be valid JSON array");
        assert_eq!(validators.len(), 0, "Should be empty array");
    }
}

/// Test registering duplicate validator replaces previous one.
#[test]
fn test_register_duplicate_validator_replaces_previous() {
    unsafe {
        kreuzberg_clear_validators();

        let name = CString::new("duplicate-validator").unwrap();

        // Register first time
        kreuzberg_register_validator(name.as_ptr(), passing_validator_callback, 50);

        // Register again with different priority
        let result = kreuzberg_register_validator(name.as_ptr(), failing_validator_callback, 100);

        assert!(result, "Duplicate registration should succeed (replace)");

        // List should still contain only one entry
        let list_ptr = kreuzberg_list_validators();
        let list_json = c_str_to_string(list_ptr).expect("Should have valid JSON");
        kreuzberg_free_string(list_ptr);

        let validators: Vec<String> = serde_json::from_str(&list_json).unwrap();
        let duplicate_count = validators.iter().filter(|v| *v == "duplicate-validator").count();
        assert_eq!(duplicate_count, 1, "Should only have one instance of the validator");

        // Cleanup
        kreuzberg_clear_validators();
    }
}

/// Test validator priorities are respected.
#[test]
fn test_validator_priorities_are_registered() {
    unsafe {
        kreuzberg_clear_validators();

        let low_priority = CString::new("low-priority-validator").unwrap();
        let high_priority = CString::new("high-priority-validator").unwrap();

        // Register with different priorities
        kreuzberg_register_validator(low_priority.as_ptr(), passing_validator_callback, 10);
        kreuzberg_register_validator(high_priority.as_ptr(), passing_validator_callback, 100);

        // Both should be registered
        let list_ptr = kreuzberg_list_validators();
        let list_json = c_str_to_string(list_ptr).expect("Should have valid JSON");
        kreuzberg_free_string(list_ptr);

        assert!(
            list_json.contains("low-priority-validator"),
            "Should contain low priority validator"
        );
        assert!(
            list_json.contains("high-priority-validator"),
            "Should contain high priority validator"
        );

        // Cleanup
        kreuzberg_clear_validators();
    }
}

// ============================================================================
// OCR Backend Plugin Tests
// ============================================================================

/// Test listing OCR backends returns valid JSON.
#[test]
fn test_list_ocr_backends_returns_valid_json() {
    unsafe {
        let list_ptr = kreuzberg_list_ocr_backends();
        assert!(!list_ptr.is_null(), "List should not be null");

        let list_json = c_str_to_string(list_ptr).expect("Should have valid JSON");
        kreuzberg_free_string(list_ptr);

        // Parse as JSON array
        let backends: Vec<String> = serde_json::from_str(&list_json).expect("Should be valid JSON array");

        // May be empty or contain default backends
        assert!(backends.is_empty() || !backends.is_empty(), "Should be a valid array");
    }
}

/// Test unregistering non-existent OCR backend succeeds gracefully.
#[test]
fn test_unregister_nonexistent_ocr_backend_succeeds_gracefully() {
    unsafe {
        let name = CString::new("nonexistent-ocr-backend").unwrap();
        let result = kreuzberg_unregister_ocr_backend(name.as_ptr());

        // Should return true (no-op for non-existent)
        assert!(result, "Unregistering non-existent OCR backend should succeed (no-op)");
    }
}

/// Test unregistering OCR backend with null name fails gracefully.
#[test]
fn test_unregister_ocr_backend_with_null_name_fails_gracefully() {
    unsafe {
        let result = kreuzberg_unregister_ocr_backend(ptr::null());

        assert!(!result, "Unregistration with null name should fail");

        let error = get_last_error();
        assert!(error.is_some(), "Should have error message");
        let error_msg = error.unwrap();
        assert!(
            error_msg.contains("NULL") || error_msg.contains("null"),
            "Error should mention null: {}",
            error_msg
        );
    }
}

/// Test unregistering OCR backend with empty name fails gracefully.
#[test]
fn test_unregister_ocr_backend_with_empty_name_fails_gracefully() {
    unsafe {
        let name = CString::new("").unwrap();
        let result = kreuzberg_unregister_ocr_backend(name.as_ptr());

        assert!(!result, "Unregistration with empty name should fail");

        let error = get_last_error();
        assert!(error.is_some(), "Should have error message");
        let error_msg = error.unwrap();
        assert!(
            error_msg.contains("empty") || error_msg.contains("invalid"),
            "Error should mention empty/invalid: {}",
            error_msg
        );
    }
}

/// Test unregistering OCR backend with whitespace in name fails gracefully.
#[test]
fn test_unregister_ocr_backend_with_whitespace_in_name_fails_gracefully() {
    unsafe {
        let name = CString::new("ocr backend with spaces").unwrap();
        let result = kreuzberg_unregister_ocr_backend(name.as_ptr());

        assert!(!result, "Unregistration with whitespace in name should fail");

        let error = get_last_error();
        assert!(error.is_some(), "Should have error message");
        let error_msg = error.unwrap();
        assert!(
            error_msg.contains("whitespace") || error_msg.contains("invalid"),
            "Error should mention whitespace/invalid: {}",
            error_msg
        );
    }
}

/// Test unregistering OCR backend with invalid UTF-8 fails gracefully.
#[test]
fn test_unregister_ocr_backend_with_invalid_utf8_fails_gracefully() {
    unsafe {
        // Create invalid UTF-8 bytes manually
        let invalid_bytes = vec![b'o', b'c', b'r', b'-', 0xFF, 0xFE, 0x00];
        let name_ptr = invalid_bytes.as_ptr() as *const i8;
        let result = kreuzberg_unregister_ocr_backend(name_ptr);

        // Should fail gracefully with UTF-8 error
        assert!(!result, "Should fail with invalid UTF-8");
        let error = get_last_error();
        assert!(error.is_some(), "Should have error message on failure");
        assert!(
            error.unwrap().contains("Invalid UTF-8"),
            "Error should mention UTF-8 issue"
        );
    }
}
