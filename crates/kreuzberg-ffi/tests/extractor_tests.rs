use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

// Import the FFI functions
unsafe extern "C" {
    fn kreuzberg_register_document_extractor(
        name: *const c_char,
        callback: unsafe extern "C" fn(*const u8, usize, *const c_char, *const c_char) -> *mut c_char,
        mime_types: *const c_char,
        priority: i32,
    ) -> bool;

    fn kreuzberg_unregister_document_extractor(name: *const c_char) -> bool;

    fn kreuzberg_list_document_extractors() -> *mut c_char;

    fn kreuzberg_last_error() -> *const c_char;

    fn kreuzberg_free_string(s: *mut c_char);
}

unsafe extern "C" fn test_extractor_callback(
    _content: *const u8,
    _content_len: usize,
    _mime_type: *const c_char,
    _config_json: *const c_char,
) -> *mut c_char {
    let result = r#"{
        "content": "test extracted content",
        "mime_type": "text/plain",
        "metadata": {}
    }"#;
    CString::new(result).unwrap().into_raw()
}

unsafe extern "C" fn failing_extractor_callback(
    _content: *const u8,
    _content_len: usize,
    _mime_type: *const c_char,
    _config_json: *const c_char,
) -> *mut c_char {
    ptr::null_mut()
}

#[test]
fn test_register_document_extractor_success() {
    unsafe {
        let name = CString::new("test-extractor").unwrap();
        let mime_types = CString::new("application/x-test,text/x-test").unwrap();

        let success =
            kreuzberg_register_document_extractor(name.as_ptr(), test_extractor_callback, mime_types.as_ptr(), 100);

        assert!(success, "Failed to register extractor");

        kreuzberg_unregister_document_extractor(name.as_ptr());
    }
}

#[test]
fn test_register_document_extractor_null_name() {
    unsafe {
        let mime_types = CString::new("application/x-test").unwrap();

        let success =
            kreuzberg_register_document_extractor(ptr::null(), test_extractor_callback, mime_types.as_ptr(), 100);

        assert!(!success, "Should fail with NULL name");

        let error = kreuzberg_last_error();
        assert!(!error.is_null());
        let error_str = CStr::from_ptr(error).to_str().unwrap();
        assert!(error_str.contains("NULL"), "Error should mention NULL: {}", error_str);
    }
}

#[test]
fn test_register_document_extractor_null_mime_types() {
    unsafe {
        let name = CString::new("test-extractor").unwrap();

        let success = kreuzberg_register_document_extractor(name.as_ptr(), test_extractor_callback, ptr::null(), 100);

        assert!(!success, "Should fail with NULL MIME types");

        let error = kreuzberg_last_error();
        assert!(!error.is_null());
        let error_str = CStr::from_ptr(error).to_str().unwrap();
        assert!(error_str.contains("MIME") || error_str.contains("NULL"));
    }
}

#[test]
fn test_register_document_extractor_empty_mime_types() {
    unsafe {
        let name = CString::new("test-extractor").unwrap();
        let mime_types = CString::new("").unwrap();

        let success =
            kreuzberg_register_document_extractor(name.as_ptr(), test_extractor_callback, mime_types.as_ptr(), 100);

        assert!(!success, "Should fail with empty MIME types");

        let error = kreuzberg_last_error();
        assert!(!error.is_null());
        let error_str = CStr::from_ptr(error).to_str().unwrap();
        assert!(error_str.contains("MIME"));
    }
}

#[test]
fn test_unregister_document_extractor_success() {
    unsafe {
        let name = CString::new("test-extractor-unregister").unwrap();
        let mime_types = CString::new("application/x-test").unwrap();

        let success =
            kreuzberg_register_document_extractor(name.as_ptr(), test_extractor_callback, mime_types.as_ptr(), 100);
        assert!(success);

        let success = kreuzberg_unregister_document_extractor(name.as_ptr());
        assert!(success, "Failed to unregister extractor");
    }
}

#[test]
fn test_unregister_document_extractor_null_name() {
    unsafe {
        let success = kreuzberg_unregister_document_extractor(ptr::null());
        assert!(!success, "Should fail with NULL name");

        let error = kreuzberg_last_error();
        assert!(!error.is_null());
        let error_str = CStr::from_ptr(error).to_str().unwrap();
        assert!(error_str.contains("NULL"));
    }
}

#[test]
fn test_unregister_nonexistent_extractor() {
    unsafe {
        let name = CString::new("nonexistent-extractor").unwrap();

        let success = kreuzberg_unregister_document_extractor(name.as_ptr());
        assert!(success, "Unregistering nonexistent extractor should succeed");
    }
}

#[test]
fn test_list_document_extractors() {
    unsafe {
        let name1 = CString::new("test-extractor-1").unwrap();
        let name2 = CString::new("test-extractor-2").unwrap();
        let mime_types = CString::new("application/x-test").unwrap();

        kreuzberg_register_document_extractor(name1.as_ptr(), test_extractor_callback, mime_types.as_ptr(), 100);
        kreuzberg_register_document_extractor(name2.as_ptr(), test_extractor_callback, mime_types.as_ptr(), 100);

        let list_ptr = kreuzberg_list_document_extractors();
        assert!(!list_ptr.is_null(), "List should not be NULL");

        let list_str = CStr::from_ptr(list_ptr).to_str().unwrap();
        assert!(list_str.contains("test-extractor-1"));
        assert!(list_str.contains("test-extractor-2"));

        kreuzberg_free_string(list_ptr);

        kreuzberg_unregister_document_extractor(name1.as_ptr());
        kreuzberg_unregister_document_extractor(name2.as_ptr());
    }
}

#[test]
fn test_register_multiple_mime_types() {
    unsafe {
        let name = CString::new("multi-mime-extractor").unwrap();
        let mime_types = CString::new("application/x-test1, text/x-test2 , image/x-test3").unwrap();

        let success =
            kreuzberg_register_document_extractor(name.as_ptr(), test_extractor_callback, mime_types.as_ptr(), 100);

        assert!(success, "Failed to register with multiple MIME types");

        kreuzberg_unregister_document_extractor(name.as_ptr());
    }
}

#[test]
fn test_register_with_different_priorities() {
    unsafe {
        let name_high = CString::new("high-priority-extractor").unwrap();
        let name_low = CString::new("low-priority-extractor").unwrap();
        let mime_types = CString::new("application/x-test").unwrap();

        let success1 = kreuzberg_register_document_extractor(
            name_high.as_ptr(),
            test_extractor_callback,
            mime_types.as_ptr(),
            200,
        );
        let success2 =
            kreuzberg_register_document_extractor(name_low.as_ptr(), test_extractor_callback, mime_types.as_ptr(), 50);

        assert!(success1 && success2, "Failed to register extractors");

        kreuzberg_unregister_document_extractor(name_high.as_ptr());
        kreuzberg_unregister_document_extractor(name_low.as_ptr());
    }
}

#[test]
fn test_invalid_utf8_name() {
    unsafe {
        let invalid_name = b"test\xFF\xFEinvalid\0";
        let mime_types = CString::new("application/x-test").unwrap();

        let success = kreuzberg_register_document_extractor(
            invalid_name.as_ptr() as *const c_char,
            test_extractor_callback,
            mime_types.as_ptr(),
            100,
        );

        assert!(!success, "Should fail with invalid UTF-8 name");

        let error = kreuzberg_last_error();
        assert!(!error.is_null());
        let error_str = CStr::from_ptr(error).to_str().unwrap();
        assert!(error_str.contains("UTF-8"));
    }
}
