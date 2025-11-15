//! C FFI bindings for Kreuzberg document intelligence library.
//!
//! Provides a C-compatible API that can be consumed by Java (Panama FFI),
//! Go (cgo), C# (P/Invoke), Zig, and other languages with C FFI support.

use std::cell::RefCell;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::Path;
use std::ptr;
use std::sync::Arc;

use async_trait::async_trait;
use kreuzberg::core::config::{ExtractionConfig, OcrConfig};
use kreuzberg::plugins::registry::get_ocr_backend_registry;
use kreuzberg::plugins::{OcrBackend, Plugin};
use kreuzberg::types::ExtractionResult;
use kreuzberg::{KreuzbergError, Result};

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
    /// Tables as JSON array (null-terminated string, or NULL if no tables, must be freed with kreuzberg_free_string)
    pub tables_json: *mut c_char,
    /// Detected languages as JSON array (null-terminated string, or NULL if not available, must be freed with kreuzberg_free_string)
    pub detected_languages_json: *mut c_char,
    /// Metadata as JSON object (null-terminated string, or NULL if no metadata, must be freed with kreuzberg_free_string)
    pub metadata_json: *mut c_char,
    /// Whether extraction was successful
    pub success: bool,
}

/// Helper function to convert ExtractionResult to CExtractionResult
fn to_c_extraction_result(result: ExtractionResult) -> std::result::Result<*mut CExtractionResult, String> {
    // Convert content to C string
    let content = CString::new(result.content)
        .map_err(|e| format!("Failed to convert content to C string: {}", e))?
        .into_raw();

    // Convert MIME type to C string
    let mime_type = CString::new(result.mime_type)
        .map(|s| s.into_raw())
        .unwrap_or_else(|e| {
            // SAFETY: Free the content we already allocated
            unsafe { drop(CString::from_raw(content)) };
            panic!("Failed to convert MIME type to C string: {}", e);
        });

    // Convert language to C string
    let language = match &result.metadata.language {
        Some(lang) => CString::new(lang.as_str())
            .ok()
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut()),
        None => ptr::null_mut(),
    };

    // Convert date to C string
    let date = match &result.metadata.date {
        Some(d) => CString::new(d.as_str())
            .ok()
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut()),
        None => ptr::null_mut(),
    };

    // Convert subject to C string
    let subject = match &result.metadata.subject {
        Some(subj) => CString::new(subj.as_str())
            .ok()
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut()),
        None => ptr::null_mut(),
    };

    // Serialize tables to JSON
    let tables_json = if !result.tables.is_empty() {
        match serde_json::to_string(&result.tables) {
            Ok(json) => CString::new(json).ok().map(|s| s.into_raw()).unwrap_or(ptr::null_mut()),
            Err(_) => ptr::null_mut(),
        }
    } else {
        ptr::null_mut()
    };

    // Serialize detected languages to JSON
    let detected_languages_json = match result.detected_languages {
        Some(langs) if !langs.is_empty() => match serde_json::to_string(&langs) {
            Ok(json) => CString::new(json).ok().map(|s| s.into_raw()).unwrap_or(ptr::null_mut()),
            Err(_) => ptr::null_mut(),
        },
        _ => ptr::null_mut(),
    };

    // Serialize metadata to JSON
    let metadata_json = match serde_json::to_string(&result.metadata) {
        Ok(json) => CString::new(json).ok().map(|s| s.into_raw()).unwrap_or(ptr::null_mut()),
        Err(_) => ptr::null_mut(),
    };

    // Allocate and return the result structure
    Ok(Box::into_raw(Box::new(CExtractionResult {
        content,
        mime_type,
        language,
        date,
        subject,
        tables_json,
        detected_languages_json,
        metadata_json,
        success: true,
    })))
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
        Ok(result) => match to_c_extraction_result(result) {
            Ok(ptr) => ptr,
            Err(e) => {
                set_last_error(e);
                ptr::null_mut()
            }
        },
        Err(e) => {
            set_last_error(e.to_string());
            ptr::null_mut()
        }
    }
}

/// Extract text and metadata from a file with custom configuration (synchronous).
///
/// # Safety
///
/// - `file_path` must be a valid null-terminated C string
/// - `config_json` must be a valid null-terminated C string containing JSON, or NULL for default config
/// - The returned pointer must be freed with `kreuzberg_free_result`
/// - Returns NULL on error (check `kreuzberg_last_error` for details)
///
/// # Example (C)
///
/// ```c
/// const char* path = "/path/to/document.pdf";
/// const char* config = "{\"force_ocr\": true, \"ocr\": {\"language\": \"deu\"}}";
/// CExtractionResult* result = kreuzberg_extract_file_sync_with_config(path, config);
/// if (result != NULL && result->success) {
///     printf("Content: %s\n", result->content);
///     kreuzberg_free_result(result);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_extract_file_sync_with_config(
    file_path: *const c_char,
    config_json: *const c_char,
) -> *mut CExtractionResult {
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

    // Parse config from JSON if provided, otherwise use default
    let config = if config_json.is_null() {
        ExtractionConfig::default()
    } else {
        // SAFETY: Caller must ensure config_json is a valid null-terminated C string
        let config_str = match unsafe { CStr::from_ptr(config_json) }.to_str() {
            Ok(s) => s,
            Err(e) => {
                set_last_error(format!("Invalid UTF-8 in config JSON: {}", e));
                return ptr::null_mut();
            }
        };

        match serde_json::from_str::<ExtractionConfig>(config_str) {
            Ok(cfg) => cfg,
            Err(e) => {
                set_last_error(format!("Failed to parse config JSON: {}", e));
                return ptr::null_mut();
            }
        }
    };

    match kreuzberg::extract_file_sync(path, None, &config) {
        Ok(result) => match to_c_extraction_result(result) {
            Ok(ptr) => ptr,
            Err(e) => {
                set_last_error(e);
                ptr::null_mut()
            }
        },
        Err(e) => {
            set_last_error(e.to_string());
            ptr::null_mut()
        }
    }
}

/// Extract text and metadata from byte array (synchronous).
///
/// # Safety
///
/// - `data` must be a valid pointer to a byte array of length `data_len`
/// - `mime_type` must be a valid null-terminated C string
/// - The returned pointer must be freed with `kreuzberg_free_result`
/// - Returns NULL on error (check `kreuzberg_last_error` for details)
///
/// # Example (C)
///
/// ```c
/// const uint8_t* data = ...; // Document bytes
/// size_t len = ...;           // Length of data
/// const char* mime = "application/pdf";
/// CExtractionResult* result = kreuzberg_extract_bytes_sync(data, len, mime);
/// if (result != NULL && result->success) {
///     printf("Content: %s\n", result->content);
///     kreuzberg_free_result(result);
/// } else {
///     const char* error = kreuzberg_last_error();
///     printf("Error: %s\n", error);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_extract_bytes_sync(
    data: *const u8,
    data_len: usize,
    mime_type: *const c_char,
) -> *mut CExtractionResult {
    clear_last_error();

    if data.is_null() {
        set_last_error("data cannot be NULL".to_string());
        return ptr::null_mut();
    }

    if mime_type.is_null() {
        set_last_error("mime_type cannot be NULL".to_string());
        return ptr::null_mut();
    }

    // SAFETY: Caller must ensure data points to a valid byte array of length data_len
    let bytes = unsafe { std::slice::from_raw_parts(data, data_len) };

    // SAFETY: Caller must ensure mime_type is a valid null-terminated C string
    let mime_str = match unsafe { CStr::from_ptr(mime_type) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            set_last_error(format!("Invalid UTF-8 in MIME type: {}", e));
            return ptr::null_mut();
        }
    };

    let config = ExtractionConfig::default();

    match kreuzberg::extract_bytes_sync(bytes, mime_str, &config) {
        Ok(result) => match to_c_extraction_result(result) {
            Ok(ptr) => ptr,
            Err(e) => {
                set_last_error(e);
                ptr::null_mut()
            }
        },
        Err(e) => {
            set_last_error(e.to_string());
            ptr::null_mut()
        }
    }
}

/// Extract text and metadata from byte array with custom configuration (synchronous).
///
/// # Safety
///
/// - `data` must be a valid pointer to a byte array of length `data_len`
/// - `mime_type` must be a valid null-terminated C string
/// - `config_json` must be a valid null-terminated C string containing JSON, or NULL for default config
/// - The returned pointer must be freed with `kreuzberg_free_result`
/// - Returns NULL on error (check `kreuzberg_last_error` for details)
///
/// # Example (C)
///
/// ```c
/// const uint8_t* data = ...; // Document bytes
/// size_t len = ...;           // Length of data
/// const char* mime = "application/pdf";
/// const char* config = "{\"force_ocr\": true, \"ocr\": {\"language\": \"deu\"}}";
/// CExtractionResult* result = kreuzberg_extract_bytes_sync_with_config(data, len, mime, config);
/// if (result != NULL && result->success) {
///     printf("Content: %s\n", result->content);
///     kreuzberg_free_result(result);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_extract_bytes_sync_with_config(
    data: *const u8,
    data_len: usize,
    mime_type: *const c_char,
    config_json: *const c_char,
) -> *mut CExtractionResult {
    clear_last_error();

    if data.is_null() {
        set_last_error("data cannot be NULL".to_string());
        return ptr::null_mut();
    }

    if mime_type.is_null() {
        set_last_error("mime_type cannot be NULL".to_string());
        return ptr::null_mut();
    }

    // SAFETY: Caller must ensure data points to a valid byte array of length data_len
    let bytes = unsafe { std::slice::from_raw_parts(data, data_len) };

    // SAFETY: Caller must ensure mime_type is a valid null-terminated C string
    let mime_str = match unsafe { CStr::from_ptr(mime_type) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            set_last_error(format!("Invalid UTF-8 in MIME type: {}", e));
            return ptr::null_mut();
        }
    };

    // Parse config from JSON if provided, otherwise use default
    let config = if config_json.is_null() {
        ExtractionConfig::default()
    } else {
        // SAFETY: Caller must ensure config_json is a valid null-terminated C string
        let config_str = match unsafe { CStr::from_ptr(config_json) }.to_str() {
            Ok(s) => s,
            Err(e) => {
                set_last_error(format!("Invalid UTF-8 in config JSON: {}", e));
                return ptr::null_mut();
            }
        };

        match serde_json::from_str::<ExtractionConfig>(config_str) {
            Ok(cfg) => cfg,
            Err(e) => {
                set_last_error(format!("Failed to parse config JSON: {}", e));
                return ptr::null_mut();
            }
        }
    };

    match kreuzberg::extract_bytes_sync(bytes, mime_str, &config) {
        Ok(result) => match to_c_extraction_result(result) {
            Ok(ptr) => ptr,
            Err(e) => {
                set_last_error(e);
                ptr::null_mut()
            }
        },
        Err(e) => {
            set_last_error(e.to_string());
            ptr::null_mut()
        }
    }
}

/// C-compatible structure for passing byte array with MIME type in batch operations
#[repr(C)]
pub struct CBytesWithMime {
    /// Pointer to byte data
    pub data: *const u8,
    /// Length of byte data
    pub data_len: usize,
    /// MIME type as null-terminated C string
    pub mime_type: *const c_char,
}

/// C-compatible structure for batch extraction results
#[repr(C)]
pub struct CBatchResult {
    /// Array of extraction results
    pub results: *mut *mut CExtractionResult,
    /// Number of results
    pub count: usize,
    /// Whether batch operation was successful
    pub success: bool,
}

/// Batch extract text and metadata from multiple files (synchronous).
///
/// # Safety
///
/// - `file_paths` must be a valid pointer to an array of null-terminated C strings
/// - `count` must be the number of file paths in the array
/// - `config_json` must be a valid null-terminated C string containing JSON, or NULL for default config
/// - The returned pointer must be freed with `kreuzberg_free_batch_result`
/// - Returns NULL on error (check `kreuzberg_last_error` for details)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_batch_extract_files_sync(
    file_paths: *const *const c_char,
    count: usize,
    config_json: *const c_char,
) -> *mut CBatchResult {
    clear_last_error();

    if file_paths.is_null() {
        set_last_error("file_paths cannot be NULL".to_string());
        return ptr::null_mut();
    }

    // Parse config from JSON if provided, otherwise use default
    let config = if config_json.is_null() {
        ExtractionConfig::default()
    } else {
        let config_str = match unsafe { CStr::from_ptr(config_json) }.to_str() {
            Ok(s) => s,
            Err(e) => {
                set_last_error(format!("Invalid UTF-8 in config JSON: {}", e));
                return ptr::null_mut();
            }
        };

        match serde_json::from_str::<ExtractionConfig>(config_str) {
            Ok(cfg) => cfg,
            Err(e) => {
                set_last_error(format!("Failed to parse config JSON: {}", e));
                return ptr::null_mut();
            }
        }
    };

    // Convert C strings to Rust paths
    let mut paths = Vec::with_capacity(count);
    for i in 0..count {
        let path_ptr = unsafe { *file_paths.add(i) };
        if path_ptr.is_null() {
            set_last_error(format!("File path at index {} is NULL", i));
            return ptr::null_mut();
        }

        let path_str = match unsafe { CStr::from_ptr(path_ptr) }.to_str() {
            Ok(s) => s,
            Err(e) => {
                set_last_error(format!("Invalid UTF-8 in file path at index {}: {}", i, e));
                return ptr::null_mut();
            }
        };

        paths.push(Path::new(path_str));
    }

    match kreuzberg::batch_extract_file_sync(paths, &config) {
        Ok(results) => {
            // Convert results to C structures
            let mut c_results = Vec::with_capacity(results.len());
            for result in results {
                match to_c_extraction_result(result) {
                    Ok(ptr) => c_results.push(ptr),
                    Err(e) => {
                        // Clean up already converted results
                        for c_res in c_results {
                            unsafe { kreuzberg_free_result(c_res) };
                        }
                        set_last_error(e);
                        return ptr::null_mut();
                    }
                }
            }

            // Allocate array for result pointers
            let results_array = c_results.into_boxed_slice();
            let results_ptr = Box::into_raw(results_array) as *mut *mut CExtractionResult;

            Box::into_raw(Box::new(CBatchResult {
                results: results_ptr,
                count,
                success: true,
            }))
        }
        Err(e) => {
            set_last_error(e.to_string());
            ptr::null_mut()
        }
    }
}

/// Batch extract text and metadata from multiple byte arrays (synchronous).
///
/// # Safety
///
/// - `items` must be a valid pointer to an array of CBytesWithMime structures
/// - `count` must be the number of items in the array
/// - `config_json` must be a valid null-terminated C string containing JSON, or NULL for default config
/// - The returned pointer must be freed with `kreuzberg_free_batch_result`
/// - Returns NULL on error (check `kreuzberg_last_error` for details)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_batch_extract_bytes_sync(
    items: *const CBytesWithMime,
    count: usize,
    config_json: *const c_char,
) -> *mut CBatchResult {
    clear_last_error();

    if items.is_null() {
        set_last_error("items cannot be NULL".to_string());
        return ptr::null_mut();
    }

    // Parse config from JSON if provided, otherwise use default
    let config = if config_json.is_null() {
        ExtractionConfig::default()
    } else {
        let config_str = match unsafe { CStr::from_ptr(config_json) }.to_str() {
            Ok(s) => s,
            Err(e) => {
                set_last_error(format!("Invalid UTF-8 in config JSON: {}", e));
                return ptr::null_mut();
            }
        };

        match serde_json::from_str::<ExtractionConfig>(config_str) {
            Ok(cfg) => cfg,
            Err(e) => {
                set_last_error(format!("Failed to parse config JSON: {}", e));
                return ptr::null_mut();
            }
        }
    };

    // Convert C structures to Rust tuples
    let mut contents = Vec::with_capacity(count);
    for i in 0..count {
        let item = unsafe { &*items.add(i) };

        if item.data.is_null() {
            set_last_error(format!("Data at index {} is NULL", i));
            return ptr::null_mut();
        }

        if item.mime_type.is_null() {
            set_last_error(format!("MIME type at index {} is NULL", i));
            return ptr::null_mut();
        }

        let bytes = unsafe { std::slice::from_raw_parts(item.data, item.data_len) };

        let mime_str = match unsafe { CStr::from_ptr(item.mime_type) }.to_str() {
            Ok(s) => s,
            Err(e) => {
                set_last_error(format!("Invalid UTF-8 in MIME type at index {}: {}", i, e));
                return ptr::null_mut();
            }
        };

        contents.push((bytes, mime_str));
    }

    match kreuzberg::batch_extract_bytes_sync(contents, &config) {
        Ok(results) => {
            // Convert results to C structures
            let mut c_results = Vec::with_capacity(results.len());
            for result in results {
                match to_c_extraction_result(result) {
                    Ok(ptr) => c_results.push(ptr),
                    Err(e) => {
                        // Clean up already converted results
                        for c_res in c_results {
                            unsafe { kreuzberg_free_result(c_res) };
                        }
                        set_last_error(e);
                        return ptr::null_mut();
                    }
                }
            }

            // Allocate array for result pointers
            let results_array = c_results.into_boxed_slice();
            let results_ptr = Box::into_raw(results_array) as *mut *mut CExtractionResult;

            Box::into_raw(Box::new(CBatchResult {
                results: results_ptr,
                count,
                success: true,
            }))
        }
        Err(e) => {
            set_last_error(e.to_string());
            ptr::null_mut()
        }
    }
}

/// Free a batch result returned by batch extraction functions.
///
/// # Safety
///
/// - `batch_result` must be a pointer previously returned by a batch extraction function
/// - `batch_result` can be NULL (no-op)
/// - `batch_result` must not be used after this call
/// - All results and strings within the batch result will be freed automatically
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_free_batch_result(batch_result: *mut CBatchResult) {
    if !batch_result.is_null() {
        let batch = unsafe { Box::from_raw(batch_result) };

        if !batch.results.is_null() {
            // Free each individual result
            let results_slice = unsafe { std::slice::from_raw_parts_mut(batch.results, batch.count) };

            for result_ptr in results_slice {
                if !result_ptr.is_null() {
                    unsafe { kreuzberg_free_result(*result_ptr) };
                }
            }

            // Free the array itself
            unsafe {
                drop(Box::from_raw(std::slice::from_raw_parts_mut(
                    batch.results,
                    batch.count,
                )))
            };
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
        if !result_box.tables_json.is_null() {
            unsafe { drop(CString::from_raw(result_box.tables_json)) };
        }
        if !result_box.detected_languages_json.is_null() {
            unsafe { drop(CString::from_raw(result_box.detected_languages_json)) };
        }
        if !result_box.metadata_json.is_null() {
            unsafe { drop(CString::from_raw(result_box.metadata_json)) };
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

/// Type alias for the OCR backend callback function.
///
/// # Parameters
///
/// - `image_bytes`: Pointer to image data
/// - `image_length`: Length of image data in bytes
/// - `config_json`: JSON-encoded OcrConfig (null-terminated string)
///
/// # Returns
///
/// Null-terminated string containing extracted text (must be freed by Rust via kreuzberg_free_string),
/// or NULL on error.
///
/// # Safety
///
/// The callback must:
/// - Not store the image_bytes pointer (it's only valid for the duration of the call)
/// - Return a valid null-terminated UTF-8 string allocated by the caller
/// - Return NULL on error (error message should be retrievable separately)
type OcrBackendCallback =
    unsafe extern "C" fn(image_bytes: *const u8, image_length: usize, config_json: *const c_char) -> *mut c_char;

/// FFI wrapper for custom OCR backends registered from Java/C.
///
/// This struct wraps a C function pointer and implements the OcrBackend trait,
/// allowing custom OCR implementations from FFI languages to be registered
/// and used within the Rust extraction pipeline.
struct FfiOcrBackend {
    name: String,
    callback: OcrBackendCallback,
}

impl FfiOcrBackend {
    fn new(name: String, callback: OcrBackendCallback) -> Self {
        Self { name, callback }
    }
}

impl Plugin for FfiOcrBackend {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> String {
        "ffi-1.0.0".to_string()
    }

    fn initialize(&self) -> Result<()> {
        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

#[async_trait]
impl OcrBackend for FfiOcrBackend {
    async fn process_image(&self, image_bytes: &[u8], config: &OcrConfig) -> Result<ExtractionResult> {
        // Serialize config to JSON
        let config_json = serde_json::to_string(config).map_err(|e| KreuzbergError::Validation {
            message: format!("Failed to serialize OCR config: {}", e),
            source: Some(Box::new(e)),
        })?;

        // Call the FFI callback (blocking operation, so spawn blocking)
        let callback = self.callback;
        let image_data = image_bytes.to_vec();
        let config_json_owned = config_json.clone();

        let result_text = tokio::task::spawn_blocking(move || {
            // Create C string inside the closure to ensure it's Send-safe
            let config_cstring = CString::new(config_json_owned).map_err(|e| KreuzbergError::Validation {
                message: format!("Failed to create C string from config JSON: {}", e),
                source: Some(Box::new(e)),
            })?;

            // SAFETY: We're passing valid pointers to the callback
            // The callback contract requires it to not store these pointers
            let result_ptr = unsafe { callback(image_data.as_ptr(), image_data.len(), config_cstring.as_ptr()) };

            if result_ptr.is_null() {
                return Err(KreuzbergError::Ocr {
                    message: "OCR backend returned NULL (operation failed)".to_string(),
                    source: None,
                });
            }

            // SAFETY: The callback contract requires returning a valid null-terminated string
            let result_cstr = unsafe { CStr::from_ptr(result_ptr) };
            let text = result_cstr
                .to_str()
                .map_err(|e| KreuzbergError::Ocr {
                    message: format!("OCR backend returned invalid UTF-8: {}", e),
                    source: Some(Box::new(e)),
                })?
                .to_string();

            // Free the string returned by the callback
            unsafe { kreuzberg_free_string(result_ptr) };

            Ok(text)
        })
        .await
        .map_err(|e| KreuzbergError::Ocr {
            message: format!("OCR backend task panicked: {}", e),
            source: Some(Box::new(e)),
        })??;

        Ok(ExtractionResult {
            content: result_text,
            mime_type: "text/plain".to_string(),
            metadata: kreuzberg::types::Metadata::default(),
            tables: vec![],
            detected_languages: None,
            chunks: None,
            images: None,
        })
    }

    fn supports_language(&self, _lang: &str) -> bool {
        // FFI backends support all languages (delegation to the foreign implementation)
        true
    }

    fn backend_type(&self) -> kreuzberg::plugins::OcrBackendType {
        kreuzberg::plugins::OcrBackendType::Custom
    }
}

/// Register a custom OCR backend via FFI callback.
///
/// # Safety
///
/// - `name` must be a valid null-terminated C string
/// - `callback` must be a valid function pointer that:
///   - Does not store the image_bytes pointer
///   - Returns a null-terminated UTF-8 string or NULL on error
///   - The returned string must be freeable by kreuzberg_free_string
/// - Returns true on success, false on error (check kreuzberg_last_error)
///
/// # Example (C)
///
/// ```c
/// char* my_ocr_backend(const uint8_t* image_bytes, size_t image_length, const char* config_json) {
///     // Implement OCR logic here
///     // Return allocated string with result, or NULL on error
///     return strdup("Extracted text");
/// }
///
/// bool success = kreuzberg_register_ocr_backend("my-ocr", my_ocr_backend);
/// if (!success) {
///     const char* error = kreuzberg_last_error();
///     printf("Failed to register: %s\n", error);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_register_ocr_backend(name: *const c_char, callback: OcrBackendCallback) -> bool {
    clear_last_error();

    if name.is_null() {
        set_last_error("Backend name cannot be NULL".to_string());
        return false;
    }

    // SAFETY: Caller must ensure name is a valid null-terminated C string
    let name_str = match unsafe { CStr::from_ptr(name) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            set_last_error(format!("Invalid UTF-8 in backend name: {}", e));
            return false;
        }
    };

    let backend = Arc::new(FfiOcrBackend::new(name_str.to_string(), callback));

    let registry = get_ocr_backend_registry();
    let mut registry_guard = match registry.write() {
        Ok(guard) => guard,
        Err(e) => {
            set_last_error(format!("Failed to acquire registry write lock: {}", e));
            return false;
        }
    };

    match registry_guard.register(backend) {
        Ok(()) => true,
        Err(e) => {
            set_last_error(format!("Failed to register OCR backend: {}", e));
            false
        }
    }
}

// ============================================================================
// PostProcessor FFI
// ============================================================================

/// Type alias for the PostProcessor callback function.
///
/// # Parameters
///
/// - `result_json`: JSON-encoded ExtractionResult (null-terminated string)
///
/// # Returns
///
/// Null-terminated JSON string containing the processed ExtractionResult
/// (must be freed by Rust via kreuzberg_free_string), or NULL on error.
///
/// # Safety
///
/// The callback must:
/// - Not store the result_json pointer (it's only valid for the duration of the call)
/// - Return a valid null-terminated UTF-8 JSON string allocated by the caller
/// - Return NULL on error (error message should be retrievable separately)
type PostProcessorCallback = unsafe extern "C" fn(result_json: *const c_char) -> *mut c_char;

/// FFI wrapper for custom PostProcessors registered from Java/C.
///
/// This struct wraps a C function pointer and implements the PostProcessor trait,
/// allowing custom post-processing implementations from FFI languages to be registered
/// and used within the Rust extraction pipeline.
struct FfiPostProcessor {
    name: String,
    callback: PostProcessorCallback,
}

impl FfiPostProcessor {
    fn new(name: String, callback: PostProcessorCallback) -> Self {
        Self { name, callback }
    }
}

impl Plugin for FfiPostProcessor {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> String {
        "ffi-1.0.0".to_string()
    }

    fn initialize(&self) -> Result<()> {
        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

#[async_trait]
impl kreuzberg::plugins::PostProcessor for FfiPostProcessor {
    async fn process(&self, result: &mut ExtractionResult, _config: &ExtractionConfig) -> Result<()> {
        // Serialize the current result to JSON
        let result_json = serde_json::to_string(&*result).map_err(|e| KreuzbergError::Validation {
            message: format!("Failed to serialize ExtractionResult: {}", e),
            source: Some(Box::new(e)),
        })?;

        // Call the FFI callback (blocking operation, so use spawn_blocking)
        let callback = self.callback;
        let processor_name = self.name.clone();
        let result_json_owned = result_json.clone();

        let processed_json = tokio::task::spawn_blocking(move || {
            // Create C string inside the closure to ensure it's Send-safe
            let result_cstring = CString::new(result_json_owned).map_err(|e| KreuzbergError::Validation {
                message: format!("Failed to create C string from result JSON: {}", e),
                source: Some(Box::new(e)),
            })?;

            // SAFETY: We're passing a valid pointer to the callback
            // The callback contract requires it to not store this pointer
            let processed_ptr = unsafe { callback(result_cstring.as_ptr()) };

            if processed_ptr.is_null() {
                return Err(KreuzbergError::Plugin {
                    message: "PostProcessor returned NULL (operation failed)".to_string(),
                    plugin_name: processor_name.clone(),
                });
            }

            // SAFETY: The callback contract requires returning a valid null-terminated string
            let processed_cstr = unsafe { CStr::from_ptr(processed_ptr) };
            let json = processed_cstr
                .to_str()
                .map_err(|e| KreuzbergError::Plugin {
                    message: format!("PostProcessor returned invalid UTF-8: {}", e),
                    plugin_name: processor_name.clone(),
                })?
                .to_string();

            // Free the string returned by the callback
            unsafe { kreuzberg_free_string(processed_ptr) };

            Ok(json)
        })
        .await
        .map_err(|e| KreuzbergError::Plugin {
            message: format!("PostProcessor task panicked: {}", e),
            plugin_name: self.name.clone(),
        })??;

        // Deserialize the processed result
        let processed_result: ExtractionResult =
            serde_json::from_str(&processed_json).map_err(|e| KreuzbergError::Plugin {
                message: format!("Failed to deserialize processed result: {}", e),
                plugin_name: self.name.clone(),
            })?;

        // Update the result with the processed data
        *result = processed_result;

        Ok(())
    }

    fn processing_stage(&self) -> kreuzberg::plugins::ProcessingStage {
        kreuzberg::plugins::ProcessingStage::Middle
    }
}

/// Register a custom PostProcessor via FFI callback.
///
/// # Safety
///
/// - `name` must be a valid null-terminated C string
/// - `callback` must be a valid function pointer that:
///   - Does not store the result_json pointer
///   - Returns a null-terminated UTF-8 JSON string or NULL on error
///   - The returned string must be freeable by kreuzberg_free_string
/// - `priority` determines the order of execution (higher priority runs first)
/// - Returns true on success, false on error (check kreuzberg_last_error)
///
/// # Example (C)
///
/// ```c
/// char* my_post_processor(const char* result_json) {
///     // Parse result_json, modify it, return JSON string
///     return strdup("{\"content\":\"PROCESSED\"}");
/// }
///
/// bool success = kreuzberg_register_post_processor("my-processor", my_post_processor, 100);
/// if (!success) {
///     const char* error = kreuzberg_last_error();
///     printf("Failed to register: %s\n", error);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_register_post_processor(
    name: *const c_char,
    callback: PostProcessorCallback,
    priority: i32,
) -> bool {
    clear_last_error();

    if name.is_null() {
        set_last_error("PostProcessor name cannot be NULL".to_string());
        return false;
    }

    // SAFETY: Caller must ensure name is a valid null-terminated C string
    let name_str = match unsafe { CStr::from_ptr(name) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            set_last_error(format!("Invalid UTF-8 in PostProcessor name: {}", e));
            return false;
        }
    };

    let processor = Arc::new(FfiPostProcessor::new(name_str.to_string(), callback));

    let registry = kreuzberg::plugins::registry::get_post_processor_registry();
    let mut registry_guard = match registry.write() {
        Ok(guard) => guard,
        Err(e) => {
            set_last_error(format!("Failed to acquire registry write lock: {}", e));
            return false;
        }
    };

    match registry_guard.register(processor, priority) {
        Ok(()) => true,
        Err(e) => {
            set_last_error(format!("Failed to register PostProcessor: {}", e));
            false
        }
    }
}

/// Unregister a PostProcessor by name.
///
/// # Safety
///
/// - `name` must be a valid null-terminated C string
/// - Returns true on success, false on error (check kreuzberg_last_error)
///
/// # Example (C)
///
/// ```c
/// bool success = kreuzberg_unregister_post_processor("my-processor");
/// if (!success) {
///     const char* error = kreuzberg_last_error();
///     printf("Failed to unregister: %s\n", error);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_unregister_post_processor(name: *const c_char) -> bool {
    clear_last_error();

    if name.is_null() {
        set_last_error("PostProcessor name cannot be NULL".to_string());
        return false;
    }

    // SAFETY: Caller must ensure name is a valid null-terminated C string
    let name_str = match unsafe { CStr::from_ptr(name) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            set_last_error(format!("Invalid UTF-8 in PostProcessor name: {}", e));
            return false;
        }
    };

    let registry = kreuzberg::plugins::registry::get_post_processor_registry();
    let mut registry_guard = match registry.write() {
        Ok(guard) => guard,
        Err(e) => {
            set_last_error(format!("Failed to acquire registry write lock: {}", e));
            return false;
        }
    };

    match registry_guard.remove(name_str) {
        Ok(()) => true,
        Err(e) => {
            set_last_error(format!("Failed to remove PostProcessor: {}", e));
            false
        }
    }
}

// ============================================================================
// Validator FFI
// ============================================================================

/// Type alias for the Validator callback function.
///
/// # Parameters
///
/// - `result_json`: JSON-encoded ExtractionResult (null-terminated string)
///
/// # Returns
///
/// Null-terminated error message string if validation fails (must be freed by Rust
/// via kreuzberg_free_string), or NULL if validation passes.
///
/// # Safety
///
/// The callback must:
/// - Not store the result_json pointer (it's only valid for the duration of the call)
/// - Return a valid null-terminated UTF-8 string (error message) if validation fails
/// - Return NULL if validation passes
/// - The returned string must be freeable by kreuzberg_free_string
type ValidatorCallback = unsafe extern "C" fn(result_json: *const c_char) -> *mut c_char;

/// FFI wrapper for custom Validators registered from Java/C.
///
/// This struct wraps a C function pointer and implements the Validator trait,
/// allowing custom validation implementations from FFI languages to be registered
/// and used within the Rust extraction pipeline.
struct FfiValidator {
    name: String,
    callback: ValidatorCallback,
    priority: i32,
}

impl FfiValidator {
    fn new(name: String, callback: ValidatorCallback, priority: i32) -> Self {
        Self {
            name,
            callback,
            priority,
        }
    }
}

impl Plugin for FfiValidator {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> String {
        "ffi-1.0.0".to_string()
    }

    fn initialize(&self) -> Result<()> {
        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

#[async_trait]
impl kreuzberg::plugins::Validator for FfiValidator {
    fn priority(&self) -> i32 {
        self.priority
    }

    async fn validate(&self, result: &ExtractionResult, _config: &ExtractionConfig) -> Result<()> {
        // Serialize the result to JSON
        let result_json = serde_json::to_string(result).map_err(|e| KreuzbergError::Validation {
            message: format!("Failed to serialize ExtractionResult: {}", e),
            source: Some(Box::new(e)),
        })?;

        // Call the FFI callback (blocking operation, so use spawn_blocking)
        let callback = self.callback;
        let validator_name = self.name.clone();
        let result_json_owned = result_json.clone();

        let error_msg = tokio::task::spawn_blocking(move || {
            // Create C string inside the closure to ensure it's Send-safe
            let result_cstring = CString::new(result_json_owned).map_err(|e| KreuzbergError::Validation {
                message: format!("Failed to create C string from result JSON: {}", e),
                source: Some(Box::new(e)),
            })?;

            // SAFETY: We're passing a valid pointer to the callback
            // The callback contract requires it to not store this pointer
            let error_ptr = unsafe { callback(result_cstring.as_ptr()) };

            if error_ptr.is_null() {
                // Validation passed
                return Ok::<Option<String>, KreuzbergError>(None);
            }

            // SAFETY: The callback contract requires returning a valid null-terminated string
            let error_cstr = unsafe { CStr::from_ptr(error_ptr) };
            let error_msg = error_cstr
                .to_str()
                .map_err(|e| KreuzbergError::Plugin {
                    message: format!("Validator returned invalid UTF-8: {}", e),
                    plugin_name: validator_name.clone(),
                })?
                .to_string();

            // Free the string returned by the callback
            unsafe { kreuzberg_free_string(error_ptr) };

            Ok(Some(error_msg))
        })
        .await
        .map_err(|e| KreuzbergError::Plugin {
            message: format!("Validator task panicked: {}", e),
            plugin_name: self.name.clone(),
        })??;

        // If there's an error message, validation failed
        if let Some(msg) = error_msg {
            return Err(KreuzbergError::Validation {
                message: msg,
                source: None,
            });
        }

        Ok(())
    }
}

/// Register a custom Validator via FFI callback.
///
/// # Safety
///
/// - `name` must be a valid null-terminated C string
/// - `callback` must be a valid function pointer that:
///   - Does not store the result_json pointer
///   - Returns a null-terminated UTF-8 string (error message) if validation fails
///   - Returns NULL if validation passes
///   - The returned string must be freeable by kreuzberg_free_string
/// - `priority` determines the order of validation (higher priority runs first)
/// - Returns true on success, false on error (check kreuzberg_last_error)
///
/// # Example (C)
///
/// ```c
/// char* my_validator(const char* result_json) {
///     // Parse result_json, validate it
///     // Return error message if validation fails, NULL if passes
///     if (invalid) {
///         return strdup("Validation failed: content too short");
///     }
///     return NULL;
/// }
///
/// bool success = kreuzberg_register_validator("my-validator", my_validator, 100);
/// if (!success) {
///     const char* error = kreuzberg_last_error();
///     printf("Failed to register: %s\n", error);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_register_validator(
    name: *const c_char,
    callback: ValidatorCallback,
    priority: i32,
) -> bool {
    clear_last_error();

    if name.is_null() {
        set_last_error("Validator name cannot be NULL".to_string());
        return false;
    }

    // SAFETY: Caller must ensure name is a valid null-terminated C string
    let name_str = match unsafe { CStr::from_ptr(name) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            set_last_error(format!("Invalid UTF-8 in Validator name: {}", e));
            return false;
        }
    };

    let validator = Arc::new(FfiValidator::new(name_str.to_string(), callback, priority));

    let registry = kreuzberg::plugins::registry::get_validator_registry();
    let mut registry_guard = match registry.write() {
        Ok(guard) => guard,
        Err(e) => {
            set_last_error(format!("Failed to acquire registry write lock: {}", e));
            return false;
        }
    };

    match registry_guard.register(validator) {
        Ok(()) => true,
        Err(e) => {
            set_last_error(format!("Failed to register Validator: {}", e));
            false
        }
    }
}

/// Unregister a Validator by name.
///
/// # Safety
///
/// - `name` must be a valid null-terminated C string
/// - Returns true on success, false on error (check kreuzberg_last_error)
///
/// # Example (C)
///
/// ```c
/// bool success = kreuzberg_unregister_validator("my-validator");
/// if (!success) {
///     const char* error = kreuzberg_last_error();
///     printf("Failed to unregister: %s\n", error);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_unregister_validator(name: *const c_char) -> bool {
    clear_last_error();

    if name.is_null() {
        set_last_error("Validator name cannot be NULL".to_string());
        return false;
    }

    // SAFETY: Caller must ensure name is a valid null-terminated C string
    let name_str = match unsafe { CStr::from_ptr(name) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            set_last_error(format!("Invalid UTF-8 in Validator name: {}", e));
            return false;
        }
    };

    let registry = kreuzberg::plugins::registry::get_validator_registry();
    let mut registry_guard = match registry.write() {
        Ok(guard) => guard,
        Err(e) => {
            set_last_error(format!("Failed to acquire registry write lock: {}", e));
            return false;
        }
    };

    match registry_guard.remove(name_str) {
        Ok(()) => true,
        Err(e) => {
            set_last_error(format!("Failed to remove Validator: {}", e));
            false
        }
    }
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
