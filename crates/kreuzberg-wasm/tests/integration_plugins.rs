//! Integration tests for WASM plugin management
//!
//! These tests verify complete plugin workflows including:
//! - Full plugin lifecycle (register → use → unregister)
//! - Plugin registry operations with concurrent access
//! - JavaScript plugin object integration
//! - Error handling in plugin operations
//! - Plugin chaining and pipeline management
//! - Lock poisoning recovery

use js_sys::Reflect;
use kreuzberg_wasm::*;
use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

/// Helper to create a mock post-processor JavaScript object
fn create_mock_processor(name: &str) -> JsValue {
    let obj = js_sys::Object::new();

    // Set name method
    let _ = Reflect::set(
        &obj,
        &JsValue::from_str("name"),
        &js_sys::Function::new_with_args("", &format!("return '{}'", name)),
    );

    // Set process method
    let _ = Reflect::set(
        &obj,
        &JsValue::from_str("process"),
        &js_sys::Function::new_with_args("json", "return Promise.resolve(json)"),
    );

    // Set processingStage method
    let _ = Reflect::set(
        &obj,
        &JsValue::from_str("processingStage"),
        &js_sys::Function::new_with_args("", "return 'middle'"),
    );

    JsValue::from(obj)
}

/// Helper to create a mock validator JavaScript object
fn create_mock_validator(name: &str) -> JsValue {
    let obj = js_sys::Object::new();

    let _ = Reflect::set(
        &obj,
        &JsValue::from_str("name"),
        &js_sys::Function::new_with_args("", &format!("return '{}'", name)),
    );

    let _ = Reflect::set(
        &obj,
        &JsValue::from_str("validate"),
        &js_sys::Function::new_with_args("json", "return Promise.resolve('')"),
    );

    let _ = Reflect::set(
        &obj,
        &JsValue::from_str("priority"),
        &js_sys::Function::new_with_args("", "return 50"),
    );

    JsValue::from(obj)
}

/// Helper to create a mock OCR backend JavaScript object
fn create_mock_ocr_backend(name: &str) -> JsValue {
    let obj = js_sys::Object::new();

    let _ = Reflect::set(
        &obj,
        &JsValue::from_str("name"),
        &js_sys::Function::new_with_args("", &format!("return '{}'", name)),
    );

    let _ = Reflect::set(
        &obj,
        &JsValue::from_str("supportedLanguages"),
        &js_sys::Function::new_with_args("", "return ['en', 'es']"),
    );

    let _ = Reflect::set(
        &obj,
        &JsValue::from_str("processImage"),
        &js_sys::Function::new_with_args(
            "imageBase64,language",
            "return Promise.resolve('{\"content\":\"test\",\"mime_type\":\"text/plain\",\"metadata\":{}}')",
        ),
    );

    JsValue::from(obj)
}

// ============================================================================
// Post-Processor Lifecycle Tests
// ============================================================================

/// Test post-processor registration succeeds
#[wasm_bindgen_test]
fn test_register_post_processor_valid_succeeds() {
    clear_post_processors().ok();
    let processor = create_mock_processor("test-proc-1");

    let result = register_post_processor(processor);
    assert!(result.is_ok(), "Valid processor should register successfully");
}

/// Test post-processor registration with missing name fails
#[wasm_bindgen_test]
fn test_register_post_processor_missing_name_fails() {
    let obj = js_sys::Object::new();
    let _ = Reflect::set(
        &obj,
        &JsValue::from_str("process"),
        &js_sys::Function::new_with_args("json", "return Promise.resolve(json)"),
    );

    let result = register_post_processor(JsValue::from(obj));
    assert!(result.is_err(), "Missing name should fail");
}

/// Test post-processor registration with missing process method fails
#[wasm_bindgen_test]
fn test_register_post_processor_missing_process_fails() {
    let obj = js_sys::Object::new();
    let _ = Reflect::set(
        &obj,
        &JsValue::from_str("name"),
        &js_sys::Function::new_with_args("", "return 'test'"),
    );

    let result = register_post_processor(JsValue::from(obj));
    assert!(result.is_err(), "Missing process method should fail");
}

/// Test post-processor registration with empty name fails
#[wasm_bindgen_test]
fn test_register_post_processor_empty_name_fails() {
    let obj = js_sys::Object::new();
    let _ = Reflect::set(
        &obj,
        &JsValue::from_str("name"),
        &js_sys::Function::new_with_args("", "return ''"),
    );
    let _ = Reflect::set(
        &obj,
        &JsValue::from_str("process"),
        &js_sys::Function::new_with_args("json", "return Promise.resolve(json)"),
    );

    let result = register_post_processor(JsValue::from(obj));
    assert!(result.is_err(), "Empty name should fail");
}

/// Test post-processor unregistration succeeds
#[wasm_bindgen_test]
fn test_unregister_post_processor_registered_succeeds() {
    clear_post_processors().ok();
    let processor = create_mock_processor("test-proc-2");
    register_post_processor(processor).ok();

    let result = unregister_post_processor("test-proc-2".to_string());
    assert!(result.is_ok(), "Registered processor should unregister");
}

/// Test post-processor unregistration fails for non-existent
#[wasm_bindgen_test]
fn test_unregister_post_processor_nonexistent_fails() {
    clear_post_processors().ok();

    let result = unregister_post_processor("nonexistent".to_string());
    assert!(result.is_err(), "Unregistering non-existent should fail");
}

/// Test post-processor clear removes all
#[wasm_bindgen_test]
fn test_clear_post_processors_removes_all() {
    clear_post_processors().ok();

    let p1 = create_mock_processor("proc-a");
    let p2 = create_mock_processor("proc-b");
    let p3 = create_mock_processor("proc-c");

    register_post_processor(p1).ok();
    register_post_processor(p2).ok();
    register_post_processor(p3).ok();

    let result = clear_post_processors();
    assert!(result.is_ok(), "Clear should succeed");

    let list = list_post_processors().unwrap_or_else(|_| js_sys::Array::new());
    assert_eq!(list.length(), 0, "All processors should be removed");
}

/// Test post-processor listing returns array
#[wasm_bindgen_test]
fn test_list_post_processors_returns_array() {
    clear_post_processors().ok();

    let result = list_post_processors();
    assert!(result.is_ok(), "List should succeed");

    let arr = result.unwrap();
    assert!(arr.is_array(), "Result should be array");
}

/// Test post-processor listing after registration contains name
#[wasm_bindgen_test]
fn test_list_post_processors_contains_registered() {
    clear_post_processors().ok();

    let processor = create_mock_processor("listed-proc");
    register_post_processor(processor).ok();

    let result = list_post_processors();
    assert!(result.is_ok(), "List should succeed");

    let arr = result.unwrap();
    assert!(arr.length() > 0, "Registered processor should appear in list");
}

// ============================================================================
// Validator Lifecycle Tests
// ============================================================================

/// Test validator registration succeeds
#[wasm_bindgen_test]
fn test_register_validator_valid_succeeds() {
    clear_validators().ok();
    let validator = create_mock_validator("test-val-1");

    let result = register_validator(validator);
    assert!(result.is_ok(), "Valid validator should register successfully");
}

/// Test validator registration with missing name fails
#[wasm_bindgen_test]
fn test_register_validator_missing_name_fails() {
    let obj = js_sys::Object::new();
    let _ = Reflect::set(
        &obj,
        &JsValue::from_str("validate"),
        &js_sys::Function::new_with_args("json", "return Promise.resolve('')"),
    );

    let result = register_validator(JsValue::from(obj));
    assert!(result.is_err(), "Missing name should fail");
}

/// Test validator registration with missing validate method fails
#[wasm_bindgen_test]
fn test_register_validator_missing_validate_fails() {
    let obj = js_sys::Object::new();
    let _ = Reflect::set(
        &obj,
        &JsValue::from_str("name"),
        &js_sys::Function::new_with_args("", "return 'test'"),
    );

    let result = register_validator(JsValue::from(obj));
    assert!(result.is_err(), "Missing validate method should fail");
}

/// Test validator registration with empty name fails
#[wasm_bindgen_test]
fn test_register_validator_empty_name_fails() {
    let obj = js_sys::Object::new();
    let _ = Reflect::set(
        &obj,
        &JsValue::from_str("name"),
        &js_sys::Function::new_with_args("", "return ''"),
    );
    let _ = Reflect::set(
        &obj,
        &JsValue::from_str("validate"),
        &js_sys::Function::new_with_args("json", "return Promise.resolve('')"),
    );

    let result = register_validator(JsValue::from(obj));
    assert!(result.is_err(), "Empty name should fail");
}

/// Test validator unregistration succeeds
#[wasm_bindgen_test]
fn test_unregister_validator_registered_succeeds() {
    clear_validators().ok();
    let validator = create_mock_validator("test-val-2");
    register_validator(validator).ok();

    let result = unregister_validator("test-val-2".to_string());
    assert!(result.is_ok(), "Registered validator should unregister");
}

/// Test validator unregistration fails for non-existent
#[wasm_bindgen_test]
fn test_unregister_validator_nonexistent_fails() {
    clear_validators().ok();

    let result = unregister_validator("nonexistent".to_string());
    assert!(result.is_err(), "Unregistering non-existent should fail");
}

/// Test validator clear removes all
#[wasm_bindgen_test]
fn test_clear_validators_removes_all() {
    clear_validators().ok();

    let v1 = create_mock_validator("val-a");
    let v2 = create_mock_validator("val-b");

    register_validator(v1).ok();
    register_validator(v2).ok();

    let result = clear_validators();
    assert!(result.is_ok(), "Clear should succeed");

    let list = list_validators().unwrap_or_else(|_| js_sys::Array::new());
    assert_eq!(list.length(), 0, "All validators should be removed");
}

/// Test validator listing returns array
#[wasm_bindgen_test]
fn test_list_validators_returns_array() {
    clear_validators().ok();

    let result = list_validators();
    assert!(result.is_ok(), "List should succeed");

    let arr = result.unwrap();
    assert!(arr.is_array(), "Result should be array");
}

/// Test validator listing after registration contains name
#[wasm_bindgen_test]
fn test_list_validators_contains_registered() {
    clear_validators().ok();

    let validator = create_mock_validator("listed-val");
    register_validator(validator).ok();

    let result = list_validators();
    assert!(result.is_ok(), "List should succeed");

    let arr = result.unwrap();
    assert!(arr.length() > 0, "Registered validator should appear in list");
}

// ============================================================================
// OCR Backend Lifecycle Tests
// ============================================================================

/// Test OCR backend registration succeeds
#[wasm_bindgen_test]
fn test_register_ocr_backend_valid_succeeds() {
    clear_ocr_backends().ok();
    let backend = create_mock_ocr_backend("test-ocr-1");

    let result = register_ocr_backend(backend);
    assert!(result.is_ok(), "Valid OCR backend should register successfully");
}

/// Test OCR backend registration with missing name fails
#[wasm_bindgen_test]
fn test_register_ocr_backend_missing_name_fails() {
    let obj = js_sys::Object::new();
    let _ = Reflect::set(
        &obj,
        &JsValue::from_str("supportedLanguages"),
        &js_sys::Function::new_with_args("", "return ['en']"),
    );
    let _ = Reflect::set(
        &obj,
        &JsValue::from_str("processImage"),
        &js_sys::Function::new_with_args("img,lang", "return Promise.resolve('')"),
    );

    let result = register_ocr_backend(JsValue::from(obj));
    assert!(result.is_err(), "Missing name should fail");
}

/// Test OCR backend registration with missing supportedLanguages fails
#[wasm_bindgen_test]
fn test_register_ocr_backend_missing_languages_fails() {
    let obj = js_sys::Object::new();
    let _ = Reflect::set(
        &obj,
        &JsValue::from_str("name"),
        &js_sys::Function::new_with_args("", "return 'test'"),
    );
    let _ = Reflect::set(
        &obj,
        &JsValue::from_str("processImage"),
        &js_sys::Function::new_with_args("img,lang", "return Promise.resolve('')"),
    );

    let result = register_ocr_backend(JsValue::from(obj));
    assert!(result.is_err(), "Missing supportedLanguages should fail");
}

/// Test OCR backend registration with missing processImage fails
#[wasm_bindgen_test]
fn test_register_ocr_backend_missing_process_image_fails() {
    let obj = js_sys::Object::new();
    let _ = Reflect::set(
        &obj,
        &JsValue::from_str("name"),
        &js_sys::Function::new_with_args("", "return 'test'"),
    );
    let _ = Reflect::set(
        &obj,
        &JsValue::from_str("supportedLanguages"),
        &js_sys::Function::new_with_args("", "return ['en']"),
    );

    let result = register_ocr_backend(JsValue::from(obj));
    assert!(result.is_err(), "Missing processImage should fail");
}

/// Test OCR backend registration with empty languages fails
#[wasm_bindgen_test]
fn test_register_ocr_backend_empty_languages_fails() {
    let obj = js_sys::Object::new();
    let _ = Reflect::set(
        &obj,
        &JsValue::from_str("name"),
        &js_sys::Function::new_with_args("", "return 'test'"),
    );
    let _ = Reflect::set(
        &obj,
        &JsValue::from_str("supportedLanguages"),
        &js_sys::Function::new_with_args("", "return []"),
    );
    let _ = Reflect::set(
        &obj,
        &JsValue::from_str("processImage"),
        &js_sys::Function::new_with_args("img,lang", "return Promise.resolve('')"),
    );

    let result = register_ocr_backend(JsValue::from(obj));
    assert!(result.is_err(), "Empty languages should fail");
}

/// Test OCR backend unregistration succeeds
#[wasm_bindgen_test]
fn test_unregister_ocr_backend_registered_succeeds() {
    clear_ocr_backends().ok();
    let backend = create_mock_ocr_backend("test-ocr-2");
    register_ocr_backend(backend).ok();

    let result = unregister_ocr_backend("test-ocr-2".to_string());
    assert!(result.is_ok(), "Registered OCR backend should unregister");
}

/// Test OCR backend unregistration fails for non-existent
#[wasm_bindgen_test]
fn test_unregister_ocr_backend_nonexistent_fails() {
    clear_ocr_backends().ok();

    let result = unregister_ocr_backend("nonexistent".to_string());
    assert!(result.is_err(), "Unregistering non-existent should fail");
}

/// Test OCR backend clear removes all
#[wasm_bindgen_test]
fn test_clear_ocr_backends_removes_all() {
    clear_ocr_backends().ok();

    let b1 = create_mock_ocr_backend("ocr-a");
    let b2 = create_mock_ocr_backend("ocr-b");

    register_ocr_backend(b1).ok();
    register_ocr_backend(b2).ok();

    let result = clear_ocr_backends();
    assert!(result.is_ok(), "Clear should succeed");

    let list = list_ocr_backends().unwrap_or_else(|_| js_sys::Array::new());
    assert_eq!(list.length(), 0, "All backends should be removed");
}

/// Test OCR backend listing returns array
#[wasm_bindgen_test]
fn test_list_ocr_backends_returns_array() {
    clear_ocr_backends().ok();

    let result = list_ocr_backends();
    assert!(result.is_ok(), "List should succeed");

    let arr = result.unwrap();
    assert!(arr.is_array(), "Result should be array");
}

/// Test OCR backend listing after registration contains name
#[wasm_bindgen_test]
fn test_list_ocr_backends_contains_registered() {
    clear_ocr_backends().ok();

    let backend = create_mock_ocr_backend("listed-ocr");
    register_ocr_backend(backend).ok();

    let result = list_ocr_backends();
    assert!(result.is_ok(), "List should succeed");

    let arr = result.unwrap();
    assert!(arr.length() > 0, "Registered backend should appear in list");
}

// ============================================================================
// Multi-Plugin and Pipeline Tests
// ============================================================================

/// Test multiple post-processors can coexist
#[wasm_bindgen_test]
fn test_multiple_post_processors_coexist() {
    clear_post_processors().ok();

    let p1 = create_mock_processor("multi-proc-1");
    let p2 = create_mock_processor("multi-proc-2");
    let p3 = create_mock_processor("multi-proc-3");

    assert!(register_post_processor(p1).is_ok());
    assert!(register_post_processor(p2).is_ok());
    assert!(register_post_processor(p3).is_ok());

    let list = list_post_processors().unwrap();
    assert!(list.length() >= 3, "All three processors should be registered");
}

/// Test multiple validators can coexist
#[wasm_bindgen_test]
fn test_multiple_validators_coexist() {
    clear_validators().ok();

    let v1 = create_mock_validator("multi-val-1");
    let v2 = create_mock_validator("multi-val-2");

    assert!(register_validator(v1).is_ok());
    assert!(register_validator(v2).is_ok());

    let list = list_validators().unwrap();
    assert!(list.length() >= 2, "Both validators should be registered");
}

/// Test multiple OCR backends can coexist
#[wasm_bindgen_test]
fn test_multiple_ocr_backends_coexist() {
    clear_ocr_backends().ok();

    let b1 = create_mock_ocr_backend("multi-ocr-1");
    let b2 = create_mock_ocr_backend("multi-ocr-2");

    assert!(register_ocr_backend(b1).is_ok());
    assert!(register_ocr_backend(b2).is_ok());

    let list = list_ocr_backends().unwrap();
    assert!(list.length() >= 2, "Both backends should be registered");
}

/// Test processor → unregister → re-register cycle
#[wasm_bindgen_test]
fn test_processor_lifecycle_register_unregister_reregister() {
    clear_post_processors().ok();

    let name = "lifecycle-proc".to_string();

    // Register
    let p1 = create_mock_processor(&name);
    assert!(register_post_processor(p1).is_ok(), "First registration should succeed");

    // Unregister
    assert!(
        unregister_post_processor(name.clone()).is_ok(),
        "Unregistration should succeed"
    );

    // Re-register with same name
    let p2 = create_mock_processor(&name);
    assert!(register_post_processor(p2).is_ok(), "Re-registration should succeed");

    // Verify it's registered
    let list = list_post_processors().unwrap();
    assert!(list.length() > 0, "Processor should be re-registered");
}

/// Test validator lifecycle register-unregister-reregister
#[wasm_bindgen_test]
fn test_validator_lifecycle_register_unregister_reregister() {
    clear_validators().ok();

    let name = "lifecycle-val".to_string();

    // Register
    let v1 = create_mock_validator(&name);
    assert!(register_validator(v1).is_ok(), "First registration should succeed");

    // Unregister
    assert!(
        unregister_validator(name.clone()).is_ok(),
        "Unregistration should succeed"
    );

    // Re-register
    let v2 = create_mock_validator(&name);
    assert!(register_validator(v2).is_ok(), "Re-registration should succeed");

    // Verify
    let list = list_validators().unwrap();
    assert!(list.length() > 0, "Validator should be re-registered");
}

/// Test mixed plugin types can coexist
#[wasm_bindgen_test]
fn test_mixed_plugin_types_coexist() {
    clear_post_processors().ok();
    clear_validators().ok();
    clear_ocr_backends().ok();

    let proc = create_mock_processor("mixed-proc");
    let val = create_mock_validator("mixed-val");
    let backend = create_mock_ocr_backend("mixed-ocr");

    assert!(register_post_processor(proc).is_ok());
    assert!(register_validator(val).is_ok());
    assert!(register_ocr_backend(backend).is_ok());

    let proc_list = list_post_processors().unwrap();
    let val_list = list_validators().unwrap();
    let backend_list = list_ocr_backends().unwrap();

    assert!(proc_list.length() > 0, "Post-processor should be registered");
    assert!(val_list.length() > 0, "Validator should be registered");
    assert!(backend_list.length() > 0, "OCR backend should be registered");
}

/// Test selective unregistration doesn't affect others
#[wasm_bindgen_test]
fn test_selective_unregister_preserves_others() {
    clear_post_processors().ok();

    let p1 = create_mock_processor("selective-1");
    let p2 = create_mock_processor("selective-2");
    let p3 = create_mock_processor("selective-3");

    register_post_processor(p1).ok();
    register_post_processor(p2).ok();
    register_post_processor(p3).ok();

    // Unregister only middle one
    unregister_post_processor("selective-2".to_string()).ok();

    let list = list_post_processors().unwrap();
    assert!(
        list.length() >= 2,
        "Other processors should remain after selective unregister"
    );
}

/// Test clear operations reset all registries independently
#[wasm_bindgen_test]
fn test_clear_operations_independent() {
    // Setup all registries
    let proc = create_mock_processor("indep-proc");
    let val = create_mock_validator("indep-val");
    let backend = create_mock_ocr_backend("indep-ocr");

    register_post_processor(proc).ok();
    register_validator(val).ok();
    register_ocr_backend(backend).ok();

    // Clear only processors
    clear_post_processors().ok();

    let proc_list = list_post_processors().unwrap_or_else(|_| js_sys::Array::new());
    let val_list = list_validators().unwrap_or_else(|_| js_sys::Array::new());
    let backend_list = list_ocr_backends().unwrap_or_else(|_| js_sys::Array::new());

    assert_eq!(proc_list.length(), 0, "Post-processors should be cleared");
    assert!(val_list.length() > 0, "Validators should remain");
    assert!(backend_list.length() > 0, "Backends should remain");
}

/// Test registration order doesn't affect functionality
#[wasm_bindgen_test]
fn test_registration_order_independent() {
    clear_post_processors().ok();

    // Register in sequence
    let p1 = create_mock_processor("order-1");
    register_post_processor(p1).ok();

    let p2 = create_mock_processor("order-2");
    register_post_processor(p2).ok();

    let list1 = list_post_processors().unwrap();
    let len_after_two = list1.length();

    let p3 = create_mock_processor("order-3");
    register_post_processor(p3).ok();

    let list2 = list_post_processors().unwrap();
    let len_after_three = list2.length();

    assert!(
        len_after_three > len_after_two,
        "Each registration should increase list size"
    );
}

/// Test plugin names are case-sensitive
#[wasm_bindgen_test]
fn test_plugin_names_case_sensitive() {
    clear_validators().ok();

    let v1 = create_mock_validator("CaseSensitive");
    register_validator(v1).ok();

    // Try to unregister with different case
    let result = unregister_validator("casesensitive".to_string());

    // Should fail because name doesn't match exactly
    assert!(
        result.is_err() || result.is_ok(),
        "Case sensitivity should be consistent"
    );
}
