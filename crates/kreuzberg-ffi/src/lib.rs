//! C FFI bindings for Kreuzberg document intelligence library.
//!
//! Provides a C-compatible API that can be consumed by Java (Panama FFI),
//! Go (cgo), C# (P/Invoke), Zig, and other languages with C FFI support.

use std::cell::RefCell;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::Path;
use std::ptr;

use kreuzberg::core::config::ExtractionConfig;

// Thread-local storage for the last error message
thread_local! {
    static LAST_ERROR: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// Set the last error message
fn set_last_error(err: String) {
    LAST_ERROR.with(|last| *last.borrow_mut() = Some(err));
}

/// Clear the last error message
fn clear_last_error() {
    LAST_ERROR.with(|last| *last.borrow_mut() = None);
}

/// C-compatible extraction result structure
#[repr(C)]
pub struct CExtractionResult {
    /// Extracted text content (null-terminated UTF-8 string, must be freed with kreuzberg_free_string)
    pub content: *mut c_char,
    /// Detected MIME type (null-terminated string, must be freed with kreuzberg_free_string)
    pub mime_type: *mut c_char,
    /// Document language (null-terminated string, or NULL if not available, must be freed with kreuzberg_free_string)
    pub language: *mut c_char,
    /// Document date (null-terminated string, or NULL if not available, must be freed with kreuzberg_free_string)
    pub date: *mut c_char,
    /// Document subject (null-terminated string, or NULL if not available, must be freed with kreuzberg_free_string)
    pub subject: *mut c_char,
    /// Whether extraction was successful
    pub success: bool,
}

/// Extract text and metadata from a file (synchronous).
///
/// # Safety
///
/// - `file_path` must be a valid null-terminated C string
/// - The returned pointer must be freed with `kreuzberg_free_result`
/// - Returns NULL on error (check `kreuzberg_last_error` for details)
///
/// # Example (C)
///
/// ```c
/// const char* path = "/path/to/document.pdf";
/// CExtractionResult* result = kreuzberg_extract_file_sync(path);
/// if (result != NULL && result->success) {
///     printf("Content: %s\n", result->content);
///     printf("MIME: %s\n", result->mime_type);
///     kreuzberg_free_result(result);
/// } else {
///     const char* error = kreuzberg_last_error();
///     printf("Error: %s\n", error);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_extract_file_sync(file_path: *const c_char) -> *mut CExtractionResult {
    clear_last_error();

    if file_path.is_null() {
        set_last_error("file_path cannot be NULL".to_string());
        return ptr::null_mut();
    }

    // SAFETY: Caller must ensure file_path is a valid null-terminated C string
    let path_str = match unsafe { CStr::from_ptr(file_path) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            set_last_error(format!("Invalid UTF-8 in file path: {}", e));
            return ptr::null_mut();
        }
    };

    let path = Path::new(path_str);
    let config = ExtractionConfig::default();

    match kreuzberg::extract_file_sync(path, None, &config) {
        Ok(result) => {
            // Convert content to C string
            let content = match CString::new(result.content) {
                Ok(s) => s.into_raw(),
                Err(e) => {
                    set_last_error(format!("Failed to convert content to C string: {}", e));
                    return ptr::null_mut();
                }
            };

            // Convert MIME type to C string
            let mime_type = match CString::new(result.mime_type) {
                Ok(s) => s.into_raw(),
                Err(e) => {
                    // SAFETY: Free the content we already allocated
                    unsafe { drop(CString::from_raw(content)) };
                    set_last_error(format!("Failed to convert MIME type to C string: {}", e));
                    return ptr::null_mut();
                }
            };

            // Convert language to C string
            let language = match result.metadata.language {
                Some(lang) => match CString::new(lang) {
                    Ok(s) => s.into_raw(),
                    Err(_) => ptr::null_mut(),
                },
                None => ptr::null_mut(),
            };

            // Convert date to C string
            let date = match result.metadata.date {
                Some(d) => match CString::new(d) {
                    Ok(s) => s.into_raw(),
                    Err(_) => ptr::null_mut(),
                },
                None => ptr::null_mut(),
            };

            // Convert subject to C string
            let subject = match result.metadata.subject {
                Some(subj) => match CString::new(subj) {
                    Ok(s) => s.into_raw(),
                    Err(_) => ptr::null_mut(),
                },
                None => ptr::null_mut(),
            };

            // Allocate and return the result structure
            Box::into_raw(Box::new(CExtractionResult {
                content,
                mime_type,
                language,
                date,
                subject,
                success: true,
            }))
        }
        Err(e) => {
            set_last_error(format!("Extraction failed: {}", e));
            ptr::null_mut()
        }
    }
}

/// Free a string returned by Kreuzberg functions.
///
/// # Safety
///
/// - `s` must be a string previously returned by a Kreuzberg function
/// - `s` can be NULL (no-op)
/// - `s` must not be used after this call
///
/// # Example (C)
///
/// ```c
/// char* str = result->content;
/// kreuzberg_free_string(str);
/// // str is now invalid
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_free_string(s: *mut c_char) {
    if !s.is_null() {
        // SAFETY: Caller must ensure s was returned by a Kreuzberg function
        unsafe { drop(CString::from_raw(s)) };
    }
}

/// Free an extraction result returned by `kreuzberg_extract_file_sync`.
///
/// # Safety
///
/// - `result` must be a pointer previously returned by `kreuzberg_extract_file_sync`
/// - `result` can be NULL (no-op)
/// - `result` must not be used after this call
/// - All string fields within the result will be freed automatically
///
/// # Example (C)
///
/// ```c
/// CExtractionResult* result = kreuzberg_extract_file_sync(path);
/// // Use result...
/// kreuzberg_free_result(result);
/// // result is now invalid
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_free_result(result: *mut CExtractionResult) {
    if !result.is_null() {
        // SAFETY: Caller must ensure result was returned by kreuzberg_extract_file_sync
        let result_box = unsafe { Box::from_raw(result) };

        // Free all string fields
        if !result_box.content.is_null() {
            unsafe { drop(CString::from_raw(result_box.content)) };
        }
        if !result_box.mime_type.is_null() {
            unsafe { drop(CString::from_raw(result_box.mime_type)) };
        }
        if !result_box.language.is_null() {
            unsafe { drop(CString::from_raw(result_box.language)) };
        }
        if !result_box.date.is_null() {
            unsafe { drop(CString::from_raw(result_box.date)) };
        }
        if !result_box.subject.is_null() {
            unsafe { drop(CString::from_raw(result_box.subject)) };
        }

        // Box drop will free the result struct itself
    }
}

/// Get the last error message from a failed operation.
///
/// # Safety
///
/// - Returns a static string that does not need to be freed
/// - Returns NULL if no error has occurred
/// - The returned string is valid until the next Kreuzberg function call on the same thread
///
/// # Example (C)
///
/// ```c
/// CExtractionResult* result = kreuzberg_extract_file_sync(path);
/// if (result == NULL) {
///     const char* error = kreuzberg_last_error();
///     if (error != NULL) {
///         printf("Error: %s\n", error);
///     }
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_last_error() -> *const c_char {
    LAST_ERROR.with(|last| match &*last.borrow() {
        Some(err) => err.as_ptr() as *const c_char,
        None => ptr::null(),
    })
}

/// Get the library version string.
///
/// # Safety
///
/// - Returns a static string that does not need to be freed
/// - The returned string is always valid
///
/// # Example (C)
///
/// ```c
/// const char* version = kreuzberg_version();
/// printf("Kreuzberg version: %s\n", version);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_version() -> *const c_char {
    concat!(env!("CARGO_PKG_VERSION"), "\0").as_ptr() as *const c_char
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn test_version() {
        unsafe {
            let version = kreuzberg_version();
            assert!(!version.is_null());
            let version_str = CStr::from_ptr(version).to_str().unwrap();
            assert!(!version_str.is_empty());
        }
    }

    #[test]
    fn test_null_path() {
        unsafe {
            let result = kreuzberg_extract_file_sync(ptr::null());
            assert!(result.is_null());

            let error = kreuzberg_last_error();
            assert!(!error.is_null());
            let error_str = CStr::from_ptr(error).to_str().unwrap();
            assert!(error_str.contains("NULL"));
        }
    }

    #[test]
    fn test_nonexistent_file() {
        unsafe {
            let path = CString::new("/nonexistent/file.pdf").unwrap();
            let result = kreuzberg_extract_file_sync(path.as_ptr());
            assert!(result.is_null());

            let error = kreuzberg_last_error();
            assert!(!error.is_null());
        }
    }
}
