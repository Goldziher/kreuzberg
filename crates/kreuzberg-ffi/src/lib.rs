//! C FFI bindings for Kreuzberg document intelligence library.
//!
//! Provides a C-compatible API that can be consumed by Java (Panama FFI),
//! Go (cgo), C# (P/Invoke), Zig, and other languages with C FFI support.

mod panic_shield;

pub use panic_shield::{
    ErrorCode, StructuredError, clear_structured_error, get_last_error_code, get_last_error_message,
    get_last_panic_context, set_structured_error,
};

use std::cell::RefCell;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::Path;
use std::ptr;
use std::sync::Arc;

use async_trait::async_trait;
use kreuzberg::core::config::{ExtractionConfig, OcrConfig};
use kreuzberg::plugins::registry::get_ocr_backend_registry;
use kreuzberg::plugins::{OcrBackend, Plugin, ProcessingStage};
use kreuzberg::types::ExtractionResult;
use kreuzberg::{KreuzbergError, Result};
#[cfg(feature = "embeddings")]
use serde::Serialize;

thread_local! {
    static LAST_ERROR_C_STRING: RefCell<Option<CString>> = const { RefCell::new(None) };
}

/// Set the last error message (convenience wrapper for backward compatibility)
fn set_last_error(err: String) {
    if let Ok(c_str) = CString::new(err.clone()) {
        LAST_ERROR_C_STRING.with(|last| *last.borrow_mut() = Some(c_str));
    }

    let structured_err = StructuredError::from_message(err, ErrorCode::GenericError);
    set_structured_error(structured_err);
}

/// Clear the last error message
fn clear_last_error() {
    LAST_ERROR_C_STRING.with(|last| *last.borrow_mut() = None);
    clear_structured_error();
}

fn string_to_c_string(value: String) -> std::result::Result<*mut c_char, String> {
    CString::new(value)
        .map(CString::into_raw)
        .map_err(|e| format!("Failed to create C string: {}", e))
}

type FfiResult<T> = std::result::Result<T, String>;

#[cfg(feature = "html")]
fn parse_extraction_config_from_json(config_str: &str) -> FfiResult<ExtractionConfig> {
    use html_to_markdown_rs::options::{
        CodeBlockStyle, ConversionOptions, HeadingStyle, HighlightStyle, ListIndentType, NewlineStyle,
        PreprocessingPreset, WhitespaceMode,
    };

    fn parse_enum<T, F>(value: Option<&serde_json::Value>, parse_fn: F) -> FfiResult<Option<T>>
    where
        F: Fn(&str) -> std::result::Result<T, String>,
    {
        if let Some(raw) = value {
            let text = raw
                .as_str()
                .ok_or_else(|| "Expected string for html_options enum field".to_string())?;
            return parse_fn(text).map(Some);
        }
        Ok(None)
    }

    fn parse_heading_style(value: &str) -> FfiResult<HeadingStyle> {
        match value.to_lowercase().as_str() {
            "atx" => Ok(HeadingStyle::Atx),
            "underlined" => Ok(HeadingStyle::Underlined),
            "atx_closed" => Ok(HeadingStyle::AtxClosed),
            other => Err(format!(
                "Invalid heading_style '{}'. Expected one of: atx, underlined, atx_closed",
                other
            )),
        }
    }

    fn parse_list_indent_type(value: &str) -> FfiResult<ListIndentType> {
        match value.to_lowercase().as_str() {
            "spaces" => Ok(ListIndentType::Spaces),
            "tabs" => Ok(ListIndentType::Tabs),
            other => Err(format!(
                "Invalid list_indent_type '{}'. Expected 'spaces' or 'tabs'",
                other
            )),
        }
    }

    fn parse_highlight_style(value: &str) -> FfiResult<HighlightStyle> {
        match value.to_lowercase().as_str() {
            "double_equal" | "==" | "highlight" => Ok(HighlightStyle::DoubleEqual),
            "html" => Ok(HighlightStyle::Html),
            "bold" => Ok(HighlightStyle::Bold),
            "none" => Ok(HighlightStyle::None),
            other => Err(format!(
                "Invalid highlight_style '{}'. Expected one of: double_equal, html, bold, none",
                other
            )),
        }
    }

    fn parse_whitespace_mode(value: &str) -> FfiResult<WhitespaceMode> {
        match value.to_lowercase().as_str() {
            "normalized" => Ok(WhitespaceMode::Normalized),
            "strict" => Ok(WhitespaceMode::Strict),
            other => Err(format!(
                "Invalid whitespace_mode '{}'. Expected 'normalized' or 'strict'",
                other
            )),
        }
    }

    fn parse_newline_style(value: &str) -> FfiResult<NewlineStyle> {
        match value.to_lowercase().as_str() {
            "spaces" => Ok(NewlineStyle::Spaces),
            "backslash" => Ok(NewlineStyle::Backslash),
            other => Err(format!(
                "Invalid newline_style '{}'. Expected 'spaces' or 'backslash'",
                other
            )),
        }
    }

    fn parse_code_block_style(value: &str) -> FfiResult<CodeBlockStyle> {
        match value.to_lowercase().as_str() {
            "indented" => Ok(CodeBlockStyle::Indented),
            "backticks" => Ok(CodeBlockStyle::Backticks),
            "tildes" => Ok(CodeBlockStyle::Tildes),
            other => Err(format!(
                "Invalid code_block_style '{}'. Expected 'indented', 'backticks', or 'tildes'",
                other
            )),
        }
    }

    fn parse_preprocessing_preset(value: &str) -> FfiResult<PreprocessingPreset> {
        match value.to_lowercase().as_str() {
            "minimal" => Ok(PreprocessingPreset::Minimal),
            "standard" => Ok(PreprocessingPreset::Standard),
            "aggressive" => Ok(PreprocessingPreset::Aggressive),
            other => Err(format!(
                "Invalid preprocessing.preset '{}'. Expected one of: minimal, standard, aggressive",
                other
            )),
        }
    }

    fn parse_html_options(value: &serde_json::Value) -> FfiResult<ConversionOptions> {
        let mut opts = ConversionOptions::default();
        let obj = value
            .as_object()
            .ok_or_else(|| "html_options must be an object".to_string())?;

        if let Some(val) = obj.get("heading_style") {
            opts.heading_style = parse_enum(Some(val), parse_heading_style)?.unwrap_or(opts.heading_style);
        }

        if let Some(val) = obj.get("list_indent_type") {
            opts.list_indent_type = parse_enum(Some(val), parse_list_indent_type)?.unwrap_or(opts.list_indent_type);
        }

        if let Some(val) = obj.get("list_indent_width") {
            opts.list_indent_width = val
                .as_u64()
                .map(|v| v as usize)
                .ok_or_else(|| "list_indent_width must be an integer".to_string())?;
        }

        if let Some(val) = obj.get("bullets") {
            opts.bullets = val
                .as_str()
                .map(str::to_string)
                .ok_or_else(|| "bullets must be a string".to_string())?;
        }

        if let Some(val) = obj.get("strong_em_symbol") {
            let symbol = val
                .as_str()
                .ok_or_else(|| "strong_em_symbol must be a string".to_string())?;
            let mut chars = symbol.chars();
            opts.strong_em_symbol = chars
                .next()
                .ok_or_else(|| "strong_em_symbol must not be empty".to_string())?;
        }

        if let Some(val) = obj.get("escape_asterisks") {
            opts.escape_asterisks = val
                .as_bool()
                .ok_or_else(|| "escape_asterisks must be a boolean".to_string())?;
        }
        if let Some(val) = obj.get("escape_underscores") {
            opts.escape_underscores = val
                .as_bool()
                .ok_or_else(|| "escape_underscores must be a boolean".to_string())?;
        }
        if let Some(val) = obj.get("escape_misc") {
            opts.escape_misc = val
                .as_bool()
                .ok_or_else(|| "escape_misc must be a boolean".to_string())?;
        }
        if let Some(val) = obj.get("escape_ascii") {
            opts.escape_ascii = val
                .as_bool()
                .ok_or_else(|| "escape_ascii must be a boolean".to_string())?;
        }

        if let Some(val) = obj.get("code_language") {
            opts.code_language = val
                .as_str()
                .map(str::to_string)
                .ok_or_else(|| "code_language must be a string".to_string())?;
        }

        if let Some(val) = obj.get("autolinks") {
            opts.autolinks = val.as_bool().ok_or_else(|| "autolinks must be a boolean".to_string())?;
        }

        if let Some(val) = obj.get("default_title") {
            opts.default_title = val
                .as_bool()
                .ok_or_else(|| "default_title must be a boolean".to_string())?;
        }

        if let Some(val) = obj.get("br_in_tables") {
            opts.br_in_tables = val
                .as_bool()
                .ok_or_else(|| "br_in_tables must be a boolean".to_string())?;
        }

        if let Some(val) = obj.get("hocr_spatial_tables") {
            opts.hocr_spatial_tables = val
                .as_bool()
                .ok_or_else(|| "hocr_spatial_tables must be a boolean".to_string())?;
        }

        if let Some(val) = obj.get("highlight_style") {
            opts.highlight_style = parse_enum(Some(val), parse_highlight_style)?.unwrap_or(opts.highlight_style);
        }

        if let Some(val) = obj.get("extract_metadata") {
            opts.extract_metadata = val
                .as_bool()
                .ok_or_else(|| "extract_metadata must be a boolean".to_string())?;
        }

        if let Some(val) = obj.get("whitespace_mode") {
            opts.whitespace_mode = parse_enum(Some(val), parse_whitespace_mode)?.unwrap_or(opts.whitespace_mode);
        }

        if let Some(val) = obj.get("strip_newlines") {
            opts.strip_newlines = val
                .as_bool()
                .ok_or_else(|| "strip_newlines must be a boolean".to_string())?;
        }

        if let Some(val) = obj.get("wrap") {
            opts.wrap = val.as_bool().ok_or_else(|| "wrap must be a boolean".to_string())?;
        }

        if let Some(val) = obj.get("wrap_width") {
            opts.wrap_width = val
                .as_u64()
                .map(|v| v as usize)
                .ok_or_else(|| "wrap_width must be an integer".to_string())?;
        }

        if let Some(val) = obj.get("convert_as_inline") {
            opts.convert_as_inline = val
                .as_bool()
                .ok_or_else(|| "convert_as_inline must be a boolean".to_string())?;
        }

        if let Some(val) = obj.get("sub_symbol") {
            opts.sub_symbol = val
                .as_str()
                .map(str::to_string)
                .ok_or_else(|| "sub_symbol must be a string".to_string())?;
        }

        if let Some(val) = obj.get("sup_symbol") {
            opts.sup_symbol = val
                .as_str()
                .map(str::to_string)
                .ok_or_else(|| "sup_symbol must be a string".to_string())?;
        }

        if let Some(val) = obj.get("newline_style") {
            opts.newline_style = parse_enum(Some(val), parse_newline_style)?.unwrap_or(opts.newline_style);
        }

        if let Some(val) = obj.get("code_block_style") {
            opts.code_block_style = parse_enum(Some(val), parse_code_block_style)?.unwrap_or(opts.code_block_style);
        }

        if let Some(val) = obj.get("keep_inline_images_in") {
            opts.keep_inline_images_in = val
                .as_array()
                .ok_or_else(|| "keep_inline_images_in must be an array".to_string())?
                .iter()
                .map(|v| {
                    v.as_str()
                        .map(str::to_string)
                        .ok_or_else(|| "keep_inline_images_in entries must be strings".to_string())
                })
                .collect::<std::result::Result<Vec<_>, _>>()?;
        }

        if let Some(val) = obj.get("encoding") {
            opts.encoding = val
                .as_str()
                .map(str::to_string)
                .ok_or_else(|| "encoding must be a string".to_string())?;
        }

        if let Some(val) = obj.get("debug") {
            opts.debug = val.as_bool().ok_or_else(|| "debug must be a boolean".to_string())?;
        }

        if let Some(val) = obj.get("strip_tags") {
            opts.strip_tags = val
                .as_array()
                .ok_or_else(|| "strip_tags must be an array".to_string())?
                .iter()
                .map(|v| {
                    v.as_str()
                        .map(str::to_string)
                        .ok_or_else(|| "strip_tags entries must be strings".to_string())
                })
                .collect::<std::result::Result<Vec<_>, _>>()?;
        }

        if let Some(val) = obj.get("preserve_tags") {
            opts.preserve_tags = val
                .as_array()
                .ok_or_else(|| "preserve_tags must be an array".to_string())?
                .iter()
                .map(|v| {
                    v.as_str()
                        .map(str::to_string)
                        .ok_or_else(|| "preserve_tags entries must be strings".to_string())
                })
                .collect::<std::result::Result<Vec<_>, _>>()?;
        }

        if let Some(val) = obj.get("preprocessing") {
            let pre = val
                .as_object()
                .ok_or_else(|| "preprocessing must be an object".to_string())?;
            let mut preprocessing = opts.preprocessing.clone();

            if let Some(v) = pre.get("enabled") {
                preprocessing.enabled = v
                    .as_bool()
                    .ok_or_else(|| "preprocessing.enabled must be a boolean".to_string())?;
            }

            if let Some(v) = pre.get("preset") {
                let preset = v
                    .as_str()
                    .ok_or_else(|| "preprocessing.preset must be a string".to_string())?;
                preprocessing.preset = parse_preprocessing_preset(preset)?;
            }

            if let Some(v) = pre.get("remove_navigation") {
                preprocessing.remove_navigation = v
                    .as_bool()
                    .ok_or_else(|| "preprocessing.remove_navigation must be a boolean".to_string())?;
            }

            if let Some(v) = pre.get("remove_forms") {
                preprocessing.remove_forms = v
                    .as_bool()
                    .ok_or_else(|| "preprocessing.remove_forms must be a boolean".to_string())?;
            }

            opts.preprocessing = preprocessing;
        }

        Ok(opts)
    }

    let value: serde_json::Value =
        serde_json::from_str(config_str).map_err(|e| format!("Failed to parse config JSON: {}", e))?;

    let html_options = value.get("html_options").map(parse_html_options).transpose()?;

    let mut config: ExtractionConfig =
        serde_json::from_value(value).map_err(|e| format!("Failed to parse config JSON: {}", e))?;

    if let Some(options) = html_options {
        config.html_options = Some(options);
    }

    Ok(config)
}

#[cfg(not(feature = "html"))]
fn parse_extraction_config_from_json(config_str: &str) -> FfiResult<ExtractionConfig> {
    let config = serde_json::from_str::<ExtractionConfig>(config_str)
        .map_err(|e| format!("Failed to parse config JSON: {}", e))?;
    Ok(config)
}

/// RAII guard for C strings to prevent memory leaks on error paths.
///
/// This wrapper ensures that if any allocation fails during the construction
/// of a CExtractionResult, all previously allocated C strings are properly freed.
/// The Drop implementation handles cleanup automatically when the guard goes out of scope.
struct CStringGuard {
    ptr: *mut c_char,
}

impl CStringGuard {
    /// Create a new guard from a CString, transferring ownership of the raw pointer
    fn new(s: CString) -> Self {
        Self { ptr: s.into_raw() }
    }

    /// Transfer ownership of the raw pointer to the caller, preventing cleanup
    fn into_raw(mut self) -> *mut c_char {
        let ptr = self.ptr;
        self.ptr = ptr::null_mut();
        ptr
    }
}

impl Drop for CStringGuard {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe { drop(CString::from_raw(self.ptr)) };
        }
    }
}

/// C-compatible extraction result structure
///
/// Must be kept in sync with the Java side's MemoryLayout definition in KreuzbergFFI.java
/// Field order: 10 pointers (8 bytes each) + 1 bool + 7 bytes padding = 88 bytes total
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
    /// Text chunks as JSON array (null-terminated string, or NULL if not available, must be freed with kreuzberg_free_string)
    pub chunks_json: *mut c_char,
    /// Extracted images as JSON array (null-terminated string, or NULL if not available, must be freed with kreuzberg_free_string)
    pub images_json: *mut c_char,
    /// Whether extraction was successful
    pub success: bool,
    /// Padding to match Java MemoryLayout (7 bytes padding to align to 8-byte boundary)
    _padding1: [u8; 7],
}

/// Helper function to convert ExtractionResult to CExtractionResult
///
/// Uses RAII guards to prevent memory leaks if any string allocation fails.
/// All allocated C strings are automatically freed if an error occurs before
/// the final result is constructed.
fn to_c_extraction_result(result: ExtractionResult) -> std::result::Result<*mut CExtractionResult, String> {
    let ExtractionResult {
        content,
        mime_type,
        metadata,
        tables,
        detected_languages,
        chunks,
        images,
    } = result;

    let content_guard =
        CStringGuard::new(CString::new(content).map_err(|e| format!("Failed to convert content to C string: {}", e))?);

    let mime_type_guard = CStringGuard::new(
        CString::new(mime_type).map_err(|e| format!("Failed to convert MIME type to C string: {}", e))?,
    );

    let language_guard = match &metadata.language {
        Some(lang) => Some(CStringGuard::new(
            CString::new(lang.as_str()).map_err(|e| format!("Failed to convert language to C string: {}", e))?,
        )),
        None => None,
    };

    let date_guard = match &metadata.date {
        Some(d) => Some(CStringGuard::new(
            CString::new(d.as_str()).map_err(|e| format!("Failed to convert date to C string: {}", e))?,
        )),
        None => None,
    };

    let subject_guard = match &metadata.subject {
        Some(subj) => Some(CStringGuard::new(
            CString::new(subj.as_str()).map_err(|e| format!("Failed to convert subject to C string: {}", e))?,
        )),
        None => None,
    };

    let tables_json_guard = if !tables.is_empty() {
        let json = serde_json::to_string(&tables).map_err(|e| format!("Failed to serialize tables to JSON: {}", e))?;
        Some(CStringGuard::new(CString::new(json).map_err(|e| {
            format!("Failed to convert tables JSON to C string: {}", e)
        })?))
    } else {
        None
    };

    let detected_languages_json_guard = match detected_languages {
        Some(langs) if !langs.is_empty() => {
            let json = serde_json::to_string(&langs)
                .map_err(|e| format!("Failed to serialize detected languages to JSON: {}", e))?;
            Some(CStringGuard::new(CString::new(json).map_err(|e| {
                format!("Failed to convert detected languages JSON to C string: {}", e)
            })?))
        }
        _ => None,
    };

    let metadata_json_guard = {
        let json =
            serde_json::to_string(&metadata).map_err(|e| format!("Failed to serialize metadata to JSON: {}", e))?;
        Some(CStringGuard::new(CString::new(json).map_err(|e| {
            format!("Failed to convert metadata JSON to C string: {}", e)
        })?))
    };

    let chunks_json_guard = match chunks {
        Some(chunks) if !chunks.is_empty() => {
            let json =
                serde_json::to_string(&chunks).map_err(|e| format!("Failed to serialize chunks to JSON: {}", e))?;
            Some(CStringGuard::new(CString::new(json).map_err(|e| {
                format!("Failed to convert chunks JSON to C string: {}", e)
            })?))
        }
        _ => None,
    };

    let images_json_guard = match images {
        Some(images) if !images.is_empty() => {
            let json =
                serde_json::to_string(&images).map_err(|e| format!("Failed to serialize images to JSON: {}", e))?;
            Some(CStringGuard::new(CString::new(json).map_err(|e| {
                format!("Failed to convert images JSON to C string: {}", e)
            })?))
        }
        _ => None,
    };

    Ok(Box::into_raw(Box::new(CExtractionResult {
        content: content_guard.into_raw(),
        mime_type: mime_type_guard.into_raw(),
        language: language_guard.map_or(ptr::null_mut(), |g| g.into_raw()),
        date: date_guard.map_or(ptr::null_mut(), |g| g.into_raw()),
        subject: subject_guard.map_or(ptr::null_mut(), |g| g.into_raw()),
        tables_json: tables_json_guard.map_or(ptr::null_mut(), |g| g.into_raw()),
        detected_languages_json: detected_languages_json_guard.map_or(ptr::null_mut(), |g| g.into_raw()),
        metadata_json: metadata_json_guard.map_or(ptr::null_mut(), |g| g.into_raw()),
        chunks_json: chunks_json_guard.map_or(ptr::null_mut(), |g| g.into_raw()),
        images_json: images_json_guard.map_or(ptr::null_mut(), |g| g.into_raw()),
        success: true,
        _padding1: [0u8; 7],
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
    ffi_panic_guard!("kreuzberg_extract_file_sync", {
        clear_last_error();

        if file_path.is_null() {
            set_last_error("file_path cannot be NULL".to_string());
            return ptr::null_mut();
        }

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
    })
}

/// Detect MIME type from a file path.
///
/// # Safety
///
/// - `file_path` must be a valid null-terminated C string
/// - The returned string must be freed with `kreuzberg_free_string`
/// - Returns NULL on error (check `kreuzberg_last_error`)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_detect_mime_type(file_path: *const c_char, check_exists: bool) -> *mut c_char {
    ffi_panic_guard!("kreuzberg_detect_mime_type", {
        clear_last_error();

        if file_path.is_null() {
            set_last_error("file_path cannot be NULL".to_string());
            return ptr::null_mut();
        }

        let path_str = match unsafe { CStr::from_ptr(file_path) }.to_str() {
            Ok(s) => s,
            Err(e) => {
                set_last_error(format!("Invalid UTF-8 in file path: {}", e));
                return ptr::null_mut();
            }
        };

        match kreuzberg::core::mime::detect_mime_type(path_str, check_exists) {
            Ok(mime) => match string_to_c_string(mime) {
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
    })
}

/// Validate that a MIME type is supported by Kreuzberg.
///
/// # Safety
///
/// - `mime_type` must be a valid null-terminated C string
/// - The returned string must be freed with `kreuzberg_free_string`
/// - Returns NULL on error (check `kreuzberg_last_error`)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_validate_mime_type(mime_type: *const c_char) -> *mut c_char {
    ffi_panic_guard!("kreuzberg_validate_mime_type", {
        clear_last_error();

        if mime_type.is_null() {
            set_last_error("mime_type cannot be NULL".to_string());
            return ptr::null_mut();
        }

        let mime_type_str = match unsafe { CStr::from_ptr(mime_type) }.to_str() {
            Ok(s) => s,
            Err(e) => {
                set_last_error(format!("Invalid UTF-8 in mime_type: {}", e));
                return ptr::null_mut();
            }
        };

        match kreuzberg::validate_mime_type(mime_type_str) {
            Ok(validated) => match string_to_c_string(validated) {
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
    })
}

#[cfg(feature = "embeddings")]
#[derive(Serialize)]
struct SerializableEmbeddingPreset<'a> {
    name: &'a str,
    chunk_size: usize,
    overlap: usize,
    model_name: String,
    dimensions: usize,
    description: &'a str,
}

/// List available embedding preset names.
///
/// # Safety
///
/// - Returned string is a JSON array and must be freed with `kreuzberg_free_string`
/// - Returns NULL on error (check `kreuzberg_last_error`)
#[cfg(feature = "embeddings")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_list_embedding_presets() -> *mut c_char {
    ffi_panic_guard!("kreuzberg_list_embedding_presets", {
        clear_last_error();

        let presets = kreuzberg::embeddings::list_presets();
        match serde_json::to_string(&presets) {
            Ok(json) => match string_to_c_string(json) {
                Ok(ptr) => ptr,
                Err(e) => {
                    set_last_error(e);
                    ptr::null_mut()
                }
            },
            Err(e) => {
                set_last_error(format!("Failed to serialize presets: {}", e));
                ptr::null_mut()
            }
        }
    })
}

/// Get a specific embedding preset by name.
///
/// # Safety
///
/// - `name` must be a valid null-terminated C string
/// - Returned string is JSON object and must be freed with `kreuzberg_free_string`
/// - Returns NULL on error (check `kreuzberg_last_error`)
#[cfg(feature = "embeddings")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_get_embedding_preset(name: *const c_char) -> *mut c_char {
    ffi_panic_guard!("kreuzberg_get_embedding_preset", {
        clear_last_error();

        if name.is_null() {
            set_last_error("preset name cannot be NULL".to_string());
            return ptr::null_mut();
        }

        let preset_name = match unsafe { CStr::from_ptr(name) }.to_str() {
            Ok(s) => s,
            Err(e) => {
                set_last_error(format!("Invalid UTF-8 in preset name: {}", e));
                return ptr::null_mut();
            }
        };

        let preset = match kreuzberg::embeddings::get_preset(preset_name) {
            Some(preset) => preset,
            None => {
                set_last_error(format!("Unknown embedding preset: {}", preset_name));
                return ptr::null_mut();
            }
        };

        let model_name = format!("{:?}", preset.model);
        let serializable = SerializableEmbeddingPreset {
            name: preset.name,
            chunk_size: preset.chunk_size,
            overlap: preset.overlap,
            model_name,
            dimensions: preset.dimensions,
            description: preset.description,
        };

        match serde_json::to_string(&serializable) {
            Ok(json) => match string_to_c_string(json) {
                Ok(ptr) => ptr,
                Err(e) => {
                    set_last_error(e);
                    ptr::null_mut()
                }
            },
            Err(e) => {
                set_last_error(format!("Failed to serialize embedding preset: {}", e));
                ptr::null_mut()
            }
        }
    })
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
    ffi_panic_guard!("kreuzberg_extract_file_sync_with_config", {
        clear_last_error();

        if file_path.is_null() {
            set_last_error("file_path cannot be NULL".to_string());
            return ptr::null_mut();
        }

        let path_str = match unsafe { CStr::from_ptr(file_path) }.to_str() {
            Ok(s) => s,
            Err(e) => {
                set_last_error(format!("Invalid UTF-8 in file path: {}", e));
                return ptr::null_mut();
            }
        };

        let path = Path::new(path_str);

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

            match parse_extraction_config_from_json(config_str) {
                Ok(cfg) => cfg,
                Err(e) => {
                    set_last_error(e);
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
    })
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
    ffi_panic_guard!("kreuzberg_extract_bytes_sync", {
        clear_last_error();

        if data.is_null() {
            set_last_error("data cannot be NULL".to_string());
            return ptr::null_mut();
        }

        if mime_type.is_null() {
            set_last_error("mime_type cannot be NULL".to_string());
            return ptr::null_mut();
        }

        let bytes = unsafe { std::slice::from_raw_parts(data, data_len) };

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
    })
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
    ffi_panic_guard!("kreuzberg_extract_bytes_sync_with_config", {
        clear_last_error();

        if data.is_null() {
            set_last_error("data cannot be NULL".to_string());
            return ptr::null_mut();
        }

        if mime_type.is_null() {
            set_last_error("mime_type cannot be NULL".to_string());
            return ptr::null_mut();
        }

        let bytes = unsafe { std::slice::from_raw_parts(data, data_len) };

        let mime_str = match unsafe { CStr::from_ptr(mime_type) }.to_str() {
            Ok(s) => s,
            Err(e) => {
                set_last_error(format!("Invalid UTF-8 in MIME type: {}", e));
                return ptr::null_mut();
            }
        };

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

            match parse_extraction_config_from_json(config_str) {
                Ok(cfg) => cfg,
                Err(e) => {
                    set_last_error(e);
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
    })
}

/// C-compatible structure for passing byte array with MIME type in batch operations
///
/// Must be kept in sync with the Java side's MemoryLayout definition in KreuzbergFFI.java
/// Field order: 1 pointer (8 bytes) + 1 usize (8 bytes) + 1 pointer (8 bytes) = 24 bytes total
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
///
/// Must be kept in sync with the Java side's MemoryLayout definition in KreuzbergFFI.java
/// Field order: 1 pointer (8 bytes) + 1 usize (8 bytes) + 1 bool + 7 bytes padding = 24 bytes total
#[repr(C)]
pub struct CBatchResult {
    /// Array of extraction results
    pub results: *mut *mut CExtractionResult,
    /// Number of results
    pub count: usize,
    /// Whether batch operation was successful
    pub success: bool,
    /// Padding to match Java MemoryLayout (7 bytes padding to align to 8-byte boundary)
    _padding2: [u8; 7],
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
    ffi_panic_guard!("kreuzberg_batch_extract_files_sync", {
        clear_last_error();

        if file_paths.is_null() {
            set_last_error("file_paths cannot be NULL".to_string());
            return ptr::null_mut();
        }

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

            match parse_extraction_config_from_json(config_str) {
                Ok(cfg) => cfg,
                Err(e) => {
                    set_last_error(e);
                    return ptr::null_mut();
                }
            }
        };

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
                let mut c_results = Vec::with_capacity(results.len());
                for result in results {
                    match to_c_extraction_result(result) {
                        Ok(ptr) => c_results.push(ptr),
                        Err(e) => {
                            for c_res in c_results {
                                unsafe { kreuzberg_free_result(c_res) };
                            }
                            set_last_error(e);
                            return ptr::null_mut();
                        }
                    }
                }

                let results_array = c_results.into_boxed_slice();
                let results_ptr = Box::into_raw(results_array) as *mut *mut CExtractionResult;

                Box::into_raw(Box::new(CBatchResult {
                    results: results_ptr,
                    count,
                    success: true,
                    _padding2: [0u8; 7],
                }))
            }
            Err(e) => {
                set_last_error(e.to_string());
                ptr::null_mut()
            }
        }
    })
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
    ffi_panic_guard!("kreuzberg_batch_extract_bytes_sync", {
        clear_last_error();

        if items.is_null() {
            set_last_error("items cannot be NULL".to_string());
            return ptr::null_mut();
        }

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

            match parse_extraction_config_from_json(config_str) {
                Ok(cfg) => cfg,
                Err(e) => {
                    set_last_error(e);
                    return ptr::null_mut();
                }
            }
        };

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
                let mut c_results = Vec::with_capacity(results.len());
                for result in results {
                    match to_c_extraction_result(result) {
                        Ok(ptr) => c_results.push(ptr),
                        Err(e) => {
                            for c_res in c_results {
                                unsafe { kreuzberg_free_result(c_res) };
                            }
                            set_last_error(e);
                            return ptr::null_mut();
                        }
                    }
                }

                let results_array = c_results.into_boxed_slice();
                let results_ptr = Box::into_raw(results_array) as *mut *mut CExtractionResult;

                Box::into_raw(Box::new(CBatchResult {
                    results: results_ptr,
                    count,
                    success: true,
                    _padding2: [0u8; 7],
                }))
            }
            Err(e) => {
                set_last_error(e.to_string());
                ptr::null_mut()
            }
        }
    })
}

/// Load an extraction configuration from a TOML/YAML/JSON file.
///
/// # Safety
///
/// - `file_path` must be a valid null-terminated C string
/// - The returned string must be freed with `kreuzberg_free_string`
/// - Returns NULL on error (check `kreuzberg_last_error`)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_load_extraction_config_from_file(file_path: *const c_char) -> *mut c_char {
    ffi_panic_guard!("kreuzberg_load_extraction_config_from_file", {
        clear_last_error();

        if file_path.is_null() {
            set_last_error("file_path cannot be NULL".to_string());
            return ptr::null_mut();
        }

        let path_str = match unsafe { CStr::from_ptr(file_path) }.to_str() {
            Ok(s) => s,
            Err(e) => {
                set_last_error(format!("Invalid UTF-8 in file path: {}", e));
                return ptr::null_mut();
            }
        };

        match ExtractionConfig::from_file(path_str) {
            Ok(config) => match serde_json::to_string(&config) {
                Ok(json) => match CString::new(json) {
                    Ok(cstr) => cstr.into_raw(),
                    Err(e) => {
                        set_last_error(format!("Failed to create C string: {}", e));
                        ptr::null_mut()
                    }
                },
                Err(e) => {
                    set_last_error(format!("Failed to serialize config to JSON: {}", e));
                    ptr::null_mut()
                }
            },
            Err(e) => {
                set_last_error(e.to_string());
                ptr::null_mut()
            }
        }
    })
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

        // NOTE: Do not free individual results here - calling code is responsible for that.
        // The Java bindings call parseAndFreeResult for each result before calling this function.
        // Freeing them here would cause a double-free.

        // Only free the results array itself
        if !batch.results.is_null() {
            unsafe {
                let _results_array = Box::from_raw(std::ptr::slice_from_raw_parts_mut(batch.results, batch.count));
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
        unsafe { drop(CString::from_raw(s)) };
    }
}

/// Clone a null-terminated string using Rust's allocator.
///
/// # Safety
///
/// - `s` must be a valid null-terminated UTF-8 string
/// - Returned pointer must be freed with `kreuzberg_free_string`
/// - Returns NULL on error (check `kreuzberg_last_error`)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_clone_string(s: *const c_char) -> *mut c_char {
    ffi_panic_guard!("kreuzberg_clone_string", {
        clear_last_error();

        if s.is_null() {
            set_last_error("Input string cannot be NULL".to_string());
            return ptr::null_mut();
        }

        let raw = match unsafe { CStr::from_ptr(s) }.to_str() {
            Ok(val) => val,
            Err(e) => {
                set_last_error(format!("Invalid UTF-8 in string: {}", e));
                return ptr::null_mut();
            }
        };

        match CString::new(raw) {
            Ok(cstr) => cstr.into_raw(),
            Err(e) => {
                set_last_error(format!("Failed to clone string: {}", e));
                ptr::null_mut()
            }
        }
    })
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
        let result_box = unsafe { Box::from_raw(result) };

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
        if !result_box.chunks_json.is_null() {
            unsafe { drop(CString::from_raw(result_box.chunks_json)) };
        }
        if !result_box.images_json.is_null() {
            unsafe { drop(CString::from_raw(result_box.images_json)) };
        }
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
    LAST_ERROR_C_STRING.with(|last| match &*last.borrow() {
        Some(c_str) => c_str.as_ptr(),
        None => ptr::null(),
    })
}

/// Get the error code for the last error.
///
/// Returns the error code as an i32. Error codes are defined in ErrorCode enum:
/// - 0: Success (no error)
/// - 1: GenericError
/// - 2: Panic
/// - 3: InvalidArgument
/// - 4: IoError
/// - 5: ParsingError
/// - 6: OcrError
/// - 7: MissingDependency
///
/// # Safety
///
/// This function is thread-safe and always safe to call.
///
/// # Example (C)
///
/// ```c
/// CExtractionResult* result = kreuzberg_extract_file_sync(path);
/// if (result == NULL) {
///     int32_t code = kreuzberg_last_error_code();
///     if (code == 2) {
///         // A panic occurred
///     }
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_last_error_code() -> i32 {
    get_last_error_code() as i32
}

/// Get the panic context for the last error (if it was a panic).
///
/// Returns a JSON string containing panic context information, or NULL if
/// the last error was not a panic.
///
/// The JSON structure contains:
/// - file: Source file where panic occurred
/// - line: Line number
/// - function: Function name
/// - message: Panic message
/// - timestamp_secs: Unix timestamp (seconds since epoch)
///
/// # Safety
///
/// The returned string must be freed with kreuzberg_free_string().
///
/// # Example (C)
///
/// ```c
/// CExtractionResult* result = kreuzberg_extract_file_sync(path);
/// if (result == NULL && kreuzberg_last_error_code() == 2) {
///     const char* context = kreuzberg_last_panic_context();
///     if (context != NULL) {
///         printf("Panic context: %s\n", context);
///         kreuzberg_free_string((char*)context);
///     }
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_last_panic_context() -> *mut c_char {
    ffi_panic_guard!("kreuzberg_last_panic_context", {
        match get_last_panic_context() {
            Some(ctx) => {
                use std::time::UNIX_EPOCH;

                let timestamp_secs = ctx
                    .timestamp
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);

                let json_value = serde_json::json!({
                    "file": ctx.file,
                    "line": ctx.line,
                    "function": ctx.function,
                    "message": ctx.message,
                    "timestamp_secs": timestamp_secs
                });

                match serde_json::to_string(&json_value) {
                    Ok(json) => match CString::new(json) {
                        Ok(c_str) => c_str.into_raw(),
                        Err(_) => ptr::null_mut(),
                    },
                    Err(_) => ptr::null_mut(),
                }
            }
            None => ptr::null_mut(),
        }
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

fn parse_languages_from_json(languages_json: *const c_char) -> FfiResult<Option<Vec<String>>> {
    if languages_json.is_null() {
        return Ok(None);
    }

    let raw = unsafe { CStr::from_ptr(languages_json) }
        .to_str()
        .map_err(|e| format!("Invalid UTF-8 in languages JSON: {}", e))?;

    if raw.trim().is_empty() {
        return Ok(None);
    }

    let langs: Vec<String> = serde_json::from_str(raw).map_err(|e| format!("Failed to parse languages JSON: {}", e))?;

    if langs.is_empty() {
        return Ok(None);
    }

    let normalized = langs
        .into_iter()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>();

    if normalized.is_empty() {
        return Ok(None);
    }

    Ok(Some(normalized))
}

/// FFI wrapper for custom OCR backends registered from Java/C.
///
/// This struct wraps a C function pointer and implements the OcrBackend trait,
/// allowing custom OCR implementations from FFI languages to be registered
/// and used within the Rust extraction pipeline.
struct FfiOcrBackend {
    name: String,
    callback: OcrBackendCallback,
    supported_languages: Option<Vec<String>>,
}

impl FfiOcrBackend {
    fn new(name: String, callback: OcrBackendCallback, supported_languages: Option<Vec<String>>) -> Self {
        Self {
            name,
            callback,
            supported_languages,
        }
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
        let config_json = serde_json::to_string(config).map_err(|e| KreuzbergError::Validation {
            message: format!("Failed to serialize OCR config: {}", e),
            source: Some(Box::new(e)),
        })?;

        let callback = self.callback;
        let image_data = image_bytes.to_vec();
        let config_json_owned = config_json.clone();

        let result_text = tokio::task::spawn_blocking(move || {
            let config_cstring = CString::new(config_json_owned).map_err(|e| KreuzbergError::Validation {
                message: format!("Failed to create C string from config JSON: {}", e),
                source: Some(Box::new(e)),
            })?;

            let result_ptr = unsafe { callback(image_data.as_ptr(), image_data.len(), config_cstring.as_ptr()) };

            if result_ptr.is_null() {
                return Err(KreuzbergError::Ocr {
                    message: "OCR backend returned NULL (operation failed)".to_string(),
                    source: None,
                });
            }

            let result_cstr = unsafe { CStr::from_ptr(result_ptr) };
            let text = result_cstr
                .to_str()
                .map_err(|e| KreuzbergError::Ocr {
                    message: format!("OCR backend returned invalid UTF-8: {}", e),
                    source: Some(Box::new(e)),
                })?
                .to_string();

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
        match &self.supported_languages {
            Some(langs) => langs.iter().any(|candidate| candidate.eq_ignore_ascii_case(_lang)),
            None => true,
        }
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
    ffi_panic_guard_bool!("kreuzberg_register_ocr_backend", {
        clear_last_error();

        if name.is_null() {
            set_last_error("Backend name cannot be NULL".to_string());
            return false;
        }

        let name_str = match unsafe { CStr::from_ptr(name) }.to_str() {
            Ok(s) => s,
            Err(e) => {
                set_last_error(format!("Invalid UTF-8 in backend name: {}", e));
                return false;
            }
        };

        if name_str.is_empty() {
            set_last_error("Plugin name cannot be empty".to_string());
            return false;
        }

        if name_str.chars().any(|c| c.is_whitespace()) {
            set_last_error("Plugin name cannot contain whitespace".to_string());
            return false;
        }

        let backend = Arc::new(FfiOcrBackend::new(name_str.to_string(), callback, None));

        let registry = get_ocr_backend_registry();
        let mut registry_guard = match registry.write() {
            Ok(guard) => guard,
            Err(e) => {
                // ~keep: Lock poisoning indicates a panic in another thread holding the lock.
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
    })
}

/// Register a custom OCR backend with explicit language support via FFI callback.
///
/// # Safety
///
/// - `languages_json` must be a null-terminated JSON array of language codes or NULL
/// - See `kreuzberg_register_ocr_backend` for additional safety notes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_register_ocr_backend_with_languages(
    name: *const c_char,
    callback: OcrBackendCallback,
    languages_json: *const c_char,
) -> bool {
    ffi_panic_guard_bool!("kreuzberg_register_ocr_backend_with_languages", {
        clear_last_error();

        if name.is_null() {
            set_last_error("Backend name cannot be NULL".to_string());
            return false;
        }

        let name_str = match unsafe { CStr::from_ptr(name) }.to_str() {
            Ok(s) => s,
            Err(e) => {
                set_last_error(format!("Invalid UTF-8 in backend name: {}", e));
                return false;
            }
        };

        if name_str.is_empty() {
            set_last_error("Plugin name cannot be empty".to_string());
            return false;
        }

        if name_str.chars().any(|c| c.is_whitespace()) {
            set_last_error("Plugin name cannot contain whitespace".to_string());
            return false;
        }

        let supported_languages = match parse_languages_from_json(languages_json) {
            Ok(langs) => langs,
            Err(e) => {
                set_last_error(e);
                return false;
            }
        };

        let backend = Arc::new(FfiOcrBackend::new(name_str.to_string(), callback, supported_languages));

        let registry = get_ocr_backend_registry();
        let mut registry_guard = match registry.write() {
            Ok(guard) => guard,
            Err(e) => {
                // ~keep: Lock poisoning indicates a panic in another thread holding the lock.
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
    })
}

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
    stage: ProcessingStage,
}

impl FfiPostProcessor {
    fn new(name: String, callback: PostProcessorCallback, stage: ProcessingStage) -> Self {
        Self { name, callback, stage }
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
        let result_json = serde_json::to_string(&*result).map_err(|e| KreuzbergError::Validation {
            message: format!("Failed to serialize ExtractionResult: {}", e),
            source: Some(Box::new(e)),
        })?;

        let callback = self.callback;
        let processor_name = self.name.clone();
        let result_json_owned = result_json.clone();

        let processed_json = tokio::task::spawn_blocking(move || {
            let result_cstring = CString::new(result_json_owned).map_err(|e| KreuzbergError::Validation {
                message: format!("Failed to create C string from result JSON: {}", e),
                source: Some(Box::new(e)),
            })?;

            let processed_ptr = unsafe { callback(result_cstring.as_ptr()) };

            if processed_ptr.is_null() {
                return Err(KreuzbergError::Plugin {
                    message: "PostProcessor returned NULL (operation failed)".to_string(),
                    plugin_name: processor_name.clone(),
                });
            }

            let processed_cstr = unsafe { CStr::from_ptr(processed_ptr) };
            let json = processed_cstr
                .to_str()
                .map_err(|e| KreuzbergError::Plugin {
                    message: format!("PostProcessor returned invalid UTF-8: {}", e),
                    plugin_name: processor_name.clone(),
                })?
                .to_string();

            unsafe { kreuzberg_free_string(processed_ptr) };

            Ok(json)
        })
        .await
        .map_err(|e| KreuzbergError::Plugin {
            message: format!("PostProcessor task panicked: {}", e),
            plugin_name: self.name.clone(),
        })??;

        let processed_result: ExtractionResult =
            serde_json::from_str(&processed_json).map_err(|e| KreuzbergError::Plugin {
                message: format!("Failed to deserialize processed result: {}", e),
                plugin_name: self.name.clone(),
            })?;

        *result = processed_result;

        Ok(())
    }

    fn processing_stage(&self) -> kreuzberg::plugins::ProcessingStage {
        self.stage
    }
}

fn parse_processing_stage(stage: Option<&str>) -> FfiResult<ProcessingStage> {
    match stage {
        Some(value) => match value.to_lowercase().as_str() {
            "early" => Ok(ProcessingStage::Early),
            "middle" => Ok(ProcessingStage::Middle),
            "late" => Ok(ProcessingStage::Late),
            other => Err(format!(
                "Invalid processing stage '{}'. Expected one of: early, middle, late",
                other
            )),
        },
        None => Ok(ProcessingStage::Middle),
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
    ffi_panic_guard_bool!("kreuzberg_register_post_processor", {
        clear_last_error();

        if name.is_null() {
            set_last_error("PostProcessor name cannot be NULL".to_string());
            return false;
        }

        let name_str = match unsafe { CStr::from_ptr(name) }.to_str() {
            Ok(s) => s,
            Err(e) => {
                set_last_error(format!("Invalid UTF-8 in PostProcessor name: {}", e));
                return false;
            }
        };

        if name_str.is_empty() {
            set_last_error("Plugin name cannot be empty".to_string());
            return false;
        }

        if name_str.chars().any(|c| c.is_whitespace()) {
            set_last_error("Plugin name cannot contain whitespace".to_string());
            return false;
        }

        let processor = Arc::new(FfiPostProcessor::new(
            name_str.to_string(),
            callback,
            ProcessingStage::Middle,
        ));

        let registry = kreuzberg::plugins::registry::get_post_processor_registry();
        let mut registry_guard = match registry.write() {
            Ok(guard) => guard,
            Err(e) => {
                // ~keep: Lock poisoning indicates a panic in another thread holding the lock.
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
    })
}

/// Register a custom PostProcessor with an explicit processing stage.
///
/// # Safety
///
/// - `name` must be a valid null-terminated C string
/// - `stage` must be a valid null-terminated C string containing "early", "middle", or "late"
/// - `callback` must be a valid function pointer that:
///   - Does not store the result_json pointer
///   - Returns a null-terminated UTF-8 JSON string or NULL on error
///   - The returned string must be freeable by kreuzberg_free_string
/// - `priority` determines the order of execution within the stage (higher priority runs first)
/// - Returns true on success, false on error (check kreuzberg_last_error)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_register_post_processor_with_stage(
    name: *const c_char,
    callback: PostProcessorCallback,
    priority: i32,
    stage: *const c_char,
) -> bool {
    ffi_panic_guard_bool!("kreuzberg_register_post_processor_with_stage", {
        clear_last_error();

        if name.is_null() {
            set_last_error("PostProcessor name cannot be NULL".to_string());
            return false;
        }

        let name_str = match unsafe { CStr::from_ptr(name) }.to_str() {
            Ok(s) => s,
            Err(e) => {
                set_last_error(format!("Invalid UTF-8 in PostProcessor name: {}", e));
                return false;
            }
        };

        if name_str.is_empty() {
            set_last_error("Plugin name cannot be empty".to_string());
            return false;
        }

        if name_str.chars().any(|c| c.is_whitespace()) {
            set_last_error("Plugin name cannot contain whitespace".to_string());
            return false;
        }

        let stage_str = if stage.is_null() {
            None
        } else {
            match unsafe { CStr::from_ptr(stage) }.to_str() {
                Ok(s) => Some(s),
                Err(e) => {
                    set_last_error(format!("Invalid UTF-8 in processing stage: {}", e));
                    return false;
                }
            }
        };

        let stage = match parse_processing_stage(stage_str) {
            Ok(stage) => stage,
            Err(e) => {
                set_last_error(e);
                return false;
            }
        };

        let processor = Arc::new(FfiPostProcessor::new(name_str.to_string(), callback, stage));

        let registry = kreuzberg::plugins::registry::get_post_processor_registry();
        let mut registry_guard = match registry.write() {
            Ok(guard) => guard,
            Err(e) => {
                // ~keep: Lock poisoning indicates a panic in another thread holding the lock.
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
    })
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
    ffi_panic_guard_bool!("kreuzberg_unregister_post_processor", {
        clear_last_error();

        if name.is_null() {
            set_last_error("PostProcessor name cannot be NULL".to_string());
            return false;
        }

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
                // ~keep: Lock poisoning indicates a panic in another thread holding the lock.
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
    })
}

/// Clear all registered PostProcessors.
///
/// # Safety
///
/// - Removes all registered processors. Subsequent extractions will run without them.
/// - Returns true on success, false on error.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_clear_post_processors() -> bool {
    ffi_panic_guard_bool!("kreuzberg_clear_post_processors", {
        clear_last_error();

        let registry = kreuzberg::plugins::registry::get_post_processor_registry();
        let mut registry_guard = match registry.write() {
            Ok(guard) => guard,
            Err(e) => {
                // ~keep: Lock poisoning indicates a panic in another thread holding the lock.
                set_last_error(format!("Failed to acquire registry write lock: {}", e));
                return false;
            }
        };

        *registry_guard = Default::default();
        true
    })
}

/// List all registered PostProcessors as a JSON array of names.
///
/// # Safety
///
/// - Returned string must be freed with `kreuzberg_free_string`.
/// - Returns NULL on error (check `kreuzberg_last_error`).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_list_post_processors() -> *mut c_char {
    ffi_panic_guard!("kreuzberg_list_post_processors", {
        clear_last_error();

        let registry = kreuzberg::plugins::registry::get_post_processor_registry();
        let registry_guard = match registry.read() {
            Ok(guard) => guard,
            Err(e) => {
                // ~keep: Lock poisoning indicates a panic in another thread holding the lock.
                set_last_error(format!("Failed to acquire registry read lock: {}", e));
                return ptr::null_mut();
            }
        };

        match serde_json::to_string(&registry_guard.list()) {
            Ok(json) => match CString::new(json) {
                Ok(cstr) => cstr.into_raw(),
                Err(e) => {
                    set_last_error(format!("Failed to create C string: {}", e));
                    ptr::null_mut()
                }
            },
            Err(e) => {
                set_last_error(format!("Failed to serialize PostProcessor list: {}", e));
                ptr::null_mut()
            }
        }
    })
}

/// Type alias for the DocumentExtractor callback function.
///
/// # Parameters
///
/// - `content`: Raw document bytes
/// - `content_len`: Length of the content array
/// - `mime_type`: MIME type of the document (null-terminated string)
/// - `config_json`: JSON-encoded ExtractionConfig (null-terminated string)
///
/// # Returns
///
/// Null-terminated JSON string containing the ExtractionResult, or NULL on error.
/// The returned string must be freeable by kreuzberg_free_string.
///
/// # Safety
///
/// The callback must:
/// - Not store the content, mime_type, or config_json pointers (only valid during the call)
/// - Return a valid null-terminated UTF-8 JSON string or NULL on error
/// - The returned string must be freeable by kreuzberg_free_string
type DocumentExtractorCallback = unsafe extern "C" fn(
    content: *const u8,
    content_len: usize,
    mime_type: *const c_char,
    config_json: *const c_char,
) -> *mut c_char;

/// FFI wrapper for custom DocumentExtractors registered from Java/C.
///
/// This struct wraps a C function pointer and implements the DocumentExtractor trait,
/// allowing custom extraction implementations from FFI languages to be registered
/// and used within the Rust extraction pipeline.
struct FfiDocumentExtractor {
    name: String,
    callback: DocumentExtractorCallback,
    #[allow(dead_code)]
    supported_types: Vec<String>,
    supported_types_static: Vec<&'static str>,
    priority: i32,
}

impl FfiDocumentExtractor {
    fn new(name: String, callback: DocumentExtractorCallback, supported_types: Vec<String>, priority: i32) -> Self {
        let supported_types_static: Vec<&'static str> = supported_types
            .iter()
            .map(|s| {
                let leaked: &'static str = Box::leak(s.clone().into_boxed_str());
                leaked
            })
            .collect();

        Self {
            name,
            callback,
            supported_types,
            supported_types_static,
            priority,
        }
    }
}

impl Plugin for FfiDocumentExtractor {
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
impl kreuzberg::plugins::DocumentExtractor for FfiDocumentExtractor {
    async fn extract_bytes(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<ExtractionResult> {
        let config_json = serde_json::to_string(config).map_err(|e| KreuzbergError::Validation {
            message: format!("Failed to serialize ExtractionConfig: {}", e),
            source: Some(Box::new(e)),
        })?;

        let callback = self.callback;
        let extractor_name = self.name.clone();
        let extractor_name_error = self.name.clone();
        let extractor_name_parse = self.name.clone();
        let content_vec = content.to_vec();
        let mime_type_owned = mime_type.to_string();
        let config_json_owned = config_json.clone();

        let result_json = tokio::task::spawn_blocking(move || {
            let mime_cstr = match CString::new(mime_type_owned.clone()) {
                Ok(s) => s,
                Err(e) => {
                    return Err(KreuzbergError::Validation {
                        message: format!("Invalid MIME type for extractor '{}': {}", extractor_name, e),
                        source: Some(Box::new(e)),
                    });
                }
            };

            let config_cstr = match CString::new(config_json_owned.clone()) {
                Ok(s) => s,
                Err(e) => {
                    return Err(KreuzbergError::Validation {
                        message: format!("Invalid config JSON for extractor '{}': {}", extractor_name, e),
                        source: Some(Box::new(e)),
                    });
                }
            };

            let result_ptr = unsafe {
                callback(
                    content_vec.as_ptr(),
                    content_vec.len(),
                    mime_cstr.as_ptr(),
                    config_cstr.as_ptr(),
                )
            };

            if result_ptr.is_null() {
                return Err(KreuzbergError::Parsing {
                    message: format!("DocumentExtractor '{}' returned NULL (callback failed)", extractor_name),
                    source: None,
                });
            }

            let result_cstr = unsafe { CString::from_raw(result_ptr) };
            let result_str = result_cstr.to_str().map_err(|e| KreuzbergError::Validation {
                message: format!("Invalid UTF-8 in result from extractor '{}': {}", extractor_name, e),
                source: Some(Box::new(e)),
            })?;

            Ok(result_str.to_string())
        })
        .await
        .map_err(|e| {
            KreuzbergError::Other(format!(
                "Task join error in extractor '{}': {}",
                extractor_name_error, e
            ))
        })??;

        serde_json::from_str(&result_json).map_err(|e| KreuzbergError::Parsing {
            message: format!(
                "Failed to deserialize ExtractionResult from extractor '{}': {}",
                extractor_name_parse, e
            ),
            source: Some(Box::new(e)),
        })
    }

    async fn extract_file(
        &self,
        path: &std::path::Path,
        mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<ExtractionResult> {
        let content = tokio::fs::read(path).await.map_err(KreuzbergError::Io)?;
        self.extract_bytes(&content, mime_type, config).await
    }

    fn supported_mime_types(&self) -> &[&str] {
        &self.supported_types_static
    }

    fn priority(&self) -> i32 {
        self.priority
    }
}

/// Register a custom DocumentExtractor via FFI callback.
///
/// # Safety
///
/// - `name` must be a valid null-terminated C string
/// - `callback` must be a valid function pointer that:
///   - Does not store the content, mime_type, or config_json pointers
///   - Returns a null-terminated UTF-8 JSON string or NULL on error
///   - The returned string must be freeable by kreuzberg_free_string
/// - `mime_types` must be a valid null-terminated C string containing comma-separated MIME types
/// - `priority` determines the order of selection (higher priority preferred)
/// - Returns true on success, false on error (check kreuzberg_last_error)
///
/// # Example (C)
///
/// ```c
/// char* my_extractor(const uint8_t* content, size_t len, const char* mime_type, const char* config) {
///     // Extract content from bytes, return JSON ExtractionResult
///     return strdup("{\"content\":\"extracted text\",\"mime_type\":\"text/plain\",\"metadata\":{}}");
/// }
///
/// bool success = kreuzberg_register_document_extractor(
///     "my-extractor",
///     my_extractor,
///     "application/x-custom,text/x-custom",
///     100
/// );
/// if (!success) {
///     const char* error = kreuzberg_last_error();
///     printf("Failed to register: %s\n", error);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_register_document_extractor(
    name: *const c_char,
    callback: DocumentExtractorCallback,
    mime_types: *const c_char,
    priority: i32,
) -> bool {
    ffi_panic_guard_bool!("kreuzberg_register_document_extractor", {
        clear_last_error();

        if name.is_null() {
            set_last_error("DocumentExtractor name cannot be NULL".to_string());
            return false;
        }

        if mime_types.is_null() {
            set_last_error("MIME types cannot be NULL".to_string());
            return false;
        }

        let name_str = match unsafe { CStr::from_ptr(name) }.to_str() {
            Ok(s) => s,
            Err(e) => {
                set_last_error(format!("Invalid UTF-8 in DocumentExtractor name: {}", e));
                return false;
            }
        };

        if name_str.is_empty() {
            set_last_error("Plugin name cannot be empty".to_string());
            return false;
        }

        if name_str.chars().any(|c| c.is_whitespace()) {
            set_last_error("Plugin name cannot contain whitespace".to_string());
            return false;
        }

        let mime_types_str = match unsafe { CStr::from_ptr(mime_types) }.to_str() {
            Ok(s) => s,
            Err(e) => {
                set_last_error(format!("Invalid UTF-8 in MIME types: {}", e));
                return false;
            }
        };

        let supported_types: Vec<String> = mime_types_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        if supported_types.is_empty() {
            set_last_error("At least one MIME type must be specified".to_string());
            return false;
        }

        let extractor = Arc::new(FfiDocumentExtractor::new(
            name_str.to_string(),
            callback,
            supported_types,
            priority,
        ));

        let registry = kreuzberg::plugins::registry::get_document_extractor_registry();
        let mut registry_guard = match registry.write() {
            Ok(guard) => guard,
            Err(e) => {
                // ~keep: Lock poisoning indicates a panic in another thread holding the lock.
                set_last_error(format!("Failed to acquire registry write lock: {}", e));
                return false;
            }
        };

        match registry_guard.register(extractor) {
            Ok(()) => true,
            Err(e) => {
                set_last_error(format!("Failed to register DocumentExtractor: {}", e));
                false
            }
        }
    })
}

/// Unregister a DocumentExtractor by name.
///
/// # Safety
///
/// - `name` must be a valid null-terminated C string
/// - Returns true on success, false on error (check kreuzberg_last_error)
///
/// # Example (C)
///
/// ```c
/// bool success = kreuzberg_unregister_document_extractor("my-extractor");
/// if (!success) {
///     const char* error = kreuzberg_last_error();
///     printf("Failed to unregister: %s\n", error);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_unregister_document_extractor(name: *const c_char) -> bool {
    ffi_panic_guard_bool!("kreuzberg_unregister_document_extractor", {
        clear_last_error();

        if name.is_null() {
            set_last_error("DocumentExtractor name cannot be NULL".to_string());
            return false;
        }

        let name_str = match unsafe { CStr::from_ptr(name) }.to_str() {
            Ok(s) => s,
            Err(e) => {
                set_last_error(format!("Invalid UTF-8 in DocumentExtractor name: {}", e));
                return false;
            }
        };

        let registry = kreuzberg::plugins::registry::get_document_extractor_registry();
        let mut registry_guard = match registry.write() {
            Ok(guard) => guard,
            Err(e) => {
                // ~keep: Lock poisoning indicates a panic in another thread holding the lock.
                set_last_error(format!("Failed to acquire registry write lock: {}", e));
                return false;
            }
        };

        match registry_guard.remove(name_str) {
            Ok(()) => true,
            Err(e) => {
                set_last_error(format!("Failed to remove DocumentExtractor: {}", e));
                false
            }
        }
    })
}

/// List all registered DocumentExtractors as a JSON array of names.
///
/// # Safety
///
/// - Returned string must be freed with `kreuzberg_free_string`.
/// - Returns NULL on error (check `kreuzberg_last_error`).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_list_document_extractors() -> *mut c_char {
    ffi_panic_guard!("kreuzberg_list_document_extractors", {
        clear_last_error();

        let registry = kreuzberg::plugins::registry::get_document_extractor_registry();
        let registry_guard = match registry.read() {
            Ok(guard) => guard,
            Err(e) => {
                // ~keep: Lock poisoning indicates a panic in another thread holding the lock.
                set_last_error(format!("Failed to acquire registry read lock: {}", e));
                return ptr::null_mut();
            }
        };

        match serde_json::to_string(&registry_guard.list()) {
            Ok(json) => match CString::new(json) {
                Ok(cstr) => cstr.into_raw(),
                Err(e) => {
                    set_last_error(format!("Failed to create C string: {}", e));
                    ptr::null_mut()
                }
            },
            Err(e) => {
                set_last_error(format!("Failed to serialize DocumentExtractor list: {}", e));
                ptr::null_mut()
            }
        }
    })
}

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
        let result_json = serde_json::to_string(result).map_err(|e| KreuzbergError::Validation {
            message: format!("Failed to serialize ExtractionResult: {}", e),
            source: Some(Box::new(e)),
        })?;

        let callback = self.callback;
        let validator_name = self.name.clone();
        let result_json_owned = result_json.clone();

        let error_msg = tokio::task::spawn_blocking(move || {
            let result_cstring = CString::new(result_json_owned).map_err(|e| KreuzbergError::Validation {
                message: format!("Failed to create C string from result JSON: {}", e),
                source: Some(Box::new(e)),
            })?;

            let error_ptr = unsafe { callback(result_cstring.as_ptr()) };

            if error_ptr.is_null() {
                return Ok::<Option<String>, KreuzbergError>(None);
            }

            let error_cstr = unsafe { CStr::from_ptr(error_ptr) };
            let error_msg = error_cstr
                .to_str()
                .map_err(|e| KreuzbergError::Plugin {
                    message: format!("Validator returned invalid UTF-8: {}", e),
                    plugin_name: validator_name.clone(),
                })?
                .to_string();

            unsafe { kreuzberg_free_string(error_ptr) };

            Ok(Some(error_msg))
        })
        .await
        .map_err(|e| KreuzbergError::Plugin {
            message: format!("Validator task panicked: {}", e),
            plugin_name: self.name.clone(),
        })??;

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
    ffi_panic_guard_bool!("kreuzberg_register_validator", {
        clear_last_error();

        if name.is_null() {
            set_last_error("Validator name cannot be NULL".to_string());
            return false;
        }

        let name_str = match unsafe { CStr::from_ptr(name) }.to_str() {
            Ok(s) => s,
            Err(e) => {
                set_last_error(format!("Invalid UTF-8 in Validator name: {}", e));
                return false;
            }
        };

        if name_str.is_empty() {
            set_last_error("Plugin name cannot be empty".to_string());
            return false;
        }

        if name_str.chars().any(|c| c.is_whitespace()) {
            set_last_error("Plugin name cannot contain whitespace".to_string());
            return false;
        }

        let validator = Arc::new(FfiValidator::new(name_str.to_string(), callback, priority));

        let registry = kreuzberg::plugins::registry::get_validator_registry();
        let mut registry_guard = match registry.write() {
            Ok(guard) => guard,
            Err(e) => {
                // ~keep: Lock poisoning indicates a panic in another thread holding the lock.
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
    })
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
    ffi_panic_guard_bool!("kreuzberg_unregister_validator", {
        clear_last_error();

        if name.is_null() {
            set_last_error("Validator name cannot be NULL".to_string());
            return false;
        }

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
                // ~keep: Lock poisoning indicates a panic in another thread holding the lock.
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
    })
}

/// Clear all registered Validators.
///
/// # Safety
///
/// - Removes all validators. Subsequent extractions will skip custom validation.
/// - Returns true on success, false on error.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_clear_validators() -> bool {
    ffi_panic_guard_bool!("kreuzberg_clear_validators", {
        clear_last_error();

        let registry = kreuzberg::plugins::registry::get_validator_registry();
        let mut registry_guard = match registry.write() {
            Ok(guard) => guard,
            Err(e) => {
                // ~keep: Lock poisoning indicates a panic in another thread holding the lock.
                set_last_error(format!("Failed to acquire registry write lock: {}", e));
                return false;
            }
        };

        *registry_guard = Default::default();
        true
    })
}

/// List all registered Validators as a JSON array of names.
///
/// # Safety
///
/// - Returned string must be freed with `kreuzberg_free_string`.
/// - Returns NULL on error (check `kreuzberg_last_error`).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_list_validators() -> *mut c_char {
    ffi_panic_guard!("kreuzberg_list_validators", {
        clear_last_error();

        let registry = kreuzberg::plugins::registry::get_validator_registry();
        let registry_guard = match registry.read() {
            Ok(guard) => guard,
            Err(e) => {
                // ~keep: Lock poisoning indicates a panic in another thread holding the lock.
                set_last_error(format!("Failed to acquire registry read lock: {}", e));
                return ptr::null_mut();
            }
        };

        match serde_json::to_string(&registry_guard.list()) {
            Ok(json) => match CString::new(json) {
                Ok(cstr) => cstr.into_raw(),
                Err(e) => {
                    set_last_error(format!("Failed to create C string: {}", e));
                    ptr::null_mut()
                }
            },
            Err(e) => {
                set_last_error(format!("Failed to serialize Validator list: {}", e));
                ptr::null_mut()
            }
        }
    })
}

/// Unregister an OCR backend by name.
///
/// # Safety
///
/// - `name` must be a valid null-terminated C string
/// - Returns true on success, false on error (check kreuzberg_last_error)
///
/// # Example (C)
///
/// ```c
/// bool success = kreuzberg_unregister_ocr_backend("custom-ocr");
/// if (!success) {
///     const char* error = kreuzberg_last_error();
///     printf("Failed to unregister: %s\n", error);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_unregister_ocr_backend(name: *const c_char) -> bool {
    ffi_panic_guard_bool!("kreuzberg_unregister_ocr_backend", {
        clear_last_error();

        if name.is_null() {
            set_last_error("OCR backend name cannot be NULL".to_string());
            return false;
        }

        let name_str = match unsafe { CStr::from_ptr(name) }.to_str() {
            Ok(s) => s,
            Err(e) => {
                set_last_error(format!("Invalid UTF-8 in OCR backend name: {}", e));
                return false;
            }
        };

        if name_str.is_empty() {
            set_last_error("OCR backend name cannot be empty".to_string());
            return false;
        }

        if name_str.chars().any(|c| c.is_whitespace()) {
            set_last_error("OCR backend name cannot contain whitespace".to_string());
            return false;
        }

        match kreuzberg::plugins::unregister_ocr_backend(name_str) {
            Ok(()) => true,
            Err(e) => {
                set_last_error(e.to_string());
                false
            }
        }
    })
}

/// List all registered OCR backends as a JSON array of names.
///
/// # Safety
///
/// - Returned string must be freed with `kreuzberg_free_string`.
/// - Returns NULL on error (check `kreuzberg_last_error`).
///
/// # Example (C)
///
/// ```c
/// char* backends = kreuzberg_list_ocr_backends();
/// if (backends == NULL) {
///     const char* error = kreuzberg_last_error();
///     printf("Failed to list backends: %s\n", error);
/// } else {
///     printf("OCR backends: %s\n", backends);
///     kreuzberg_free_string(backends);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_list_ocr_backends() -> *mut c_char {
    ffi_panic_guard!("kreuzberg_list_ocr_backends", {
        clear_last_error();

        match kreuzberg::plugins::list_ocr_backends() {
            Ok(backends) => match serde_json::to_string(&backends) {
                Ok(json) => match CString::new(json) {
                    Ok(cstr) => cstr.into_raw(),
                    Err(e) => {
                        set_last_error(format!("Failed to create C string: {}", e));
                        ptr::null_mut()
                    }
                },
                Err(e) => {
                    set_last_error(format!("Failed to serialize OCR backend list: {}", e));
                    ptr::null_mut()
                }
            },
            Err(e) => {
                set_last_error(e.to_string());
                ptr::null_mut()
            }
        }
    })
}

/// Clear all registered OCR backends.
///
/// # Safety
///
/// - Removes all registered OCR backends. Subsequent extractions will use only built-in backends.
/// - Returns true on success, false on error.
///
/// # Example (C)
///
/// ```c
/// bool success = kreuzberg_clear_ocr_backends();
/// if (!success) {
///     const char* error = kreuzberg_last_error();
///     printf("Failed to clear OCR backends: %s\n", error);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_clear_ocr_backends() -> bool {
    ffi_panic_guard_bool!("kreuzberg_clear_ocr_backends", {
        clear_last_error();

        match kreuzberg::plugins::clear_ocr_backends() {
            Ok(()) => true,
            Err(e) => {
                set_last_error(e.to_string());
                false
            }
        }
    })
}

/// Clear all registered DocumentExtractors.
///
/// # Safety
///
/// - Removes all registered extractors. Subsequent extractions will use only built-in extractors.
/// - Returns true on success, false on error.
///
/// # Example (C)
///
/// ```c
/// bool success = kreuzberg_clear_document_extractors();
/// if (!success) {
///     const char* error = kreuzberg_last_error();
///     printf("Failed to clear document extractors: %s\n", error);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_clear_document_extractors() -> bool {
    ffi_panic_guard_bool!("kreuzberg_clear_document_extractors", {
        clear_last_error();

        let registry = kreuzberg::plugins::registry::get_document_extractor_registry();
        let mut registry_guard = match registry.write() {
            Ok(guard) => guard,
            Err(e) => {
                // ~keep: Lock poisoning indicates a panic in another thread holding the lock.
                set_last_error(format!("Failed to acquire registry write lock: {}", e));
                return false;
            }
        };

        *registry_guard = Default::default();
        true
    })
}

/// Detect MIME type from raw bytes.
///
/// # Safety
///
/// - `bytes` must be a valid pointer to byte data
/// - `len` must be the correct length of the byte array
/// - The returned string must be freed with `kreuzberg_free_string`
/// - Returns NULL on error (check `kreuzberg_last_error`)
///
/// # Example (C)
///
/// ```c
/// const char* pdf_bytes = "%PDF-1.4\n";
/// char* mime = kreuzberg_detect_mime_type_from_bytes((const uint8_t*)pdf_bytes, strlen(pdf_bytes));
/// if (mime == NULL) {
///     const char* error = kreuzberg_last_error();
///     printf("Failed to detect MIME type: %s\n", error);
/// } else {
///     printf("MIME type: %s\n", mime);
///     kreuzberg_free_string(mime);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_detect_mime_type_from_bytes(bytes: *const u8, len: usize) -> *mut c_char {
    ffi_panic_guard!("kreuzberg_detect_mime_type_from_bytes", {
        clear_last_error();

        if bytes.is_null() {
            set_last_error("bytes cannot be NULL".to_string());
            return ptr::null_mut();
        }

        let slice = unsafe { std::slice::from_raw_parts(bytes, len) };

        match kreuzberg::core::mime::detect_mime_type_from_bytes(slice) {
            Ok(mime) => match string_to_c_string(mime) {
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
    })
}

/// Detect MIME type from file path (checks extension and reads file content).
///
/// # Safety
///
/// - `file_path` must be a valid null-terminated C string
/// - The returned string must be freed with `kreuzberg_free_string`
/// - Returns NULL on error (check `kreuzberg_last_error`)
///
/// # Example (C)
///
/// ```c
/// char* mime = kreuzberg_detect_mime_type_from_path("document.pdf");
/// if (mime == NULL) {
///     const char* error = kreuzberg_last_error();
///     printf("Failed to detect MIME type: %s\n", error);
/// } else {
///     printf("MIME type: %s\n", mime);
///     kreuzberg_free_string(mime);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_detect_mime_type_from_path(file_path: *const c_char) -> *mut c_char {
    ffi_panic_guard!("kreuzberg_detect_mime_type_from_path", {
        clear_last_error();

        if file_path.is_null() {
            set_last_error("file_path cannot be NULL".to_string());
            return ptr::null_mut();
        }

        let path_str = match unsafe { CStr::from_ptr(file_path) }.to_str() {
            Ok(s) => s,
            Err(e) => {
                set_last_error(format!("Invalid UTF-8 in file path: {}", e));
                return ptr::null_mut();
            }
        };

        match kreuzberg::core::mime::detect_mime_type(path_str, true) {
            Ok(mime) => match string_to_c_string(mime) {
                Ok(ptr) => ptr,
                Err(e) => {
                    set_last_error(e);
                    ptr::null_mut()
                }
            },
            Err(e) => {
                // ~keep: IO errors from file operations should bubble up as they indicate
                set_last_error(e.to_string());
                ptr::null_mut()
            }
        }
    })
}

/// Get file extensions for a MIME type.
///
/// # Safety
///
/// - `mime_type` must be a valid null-terminated C string
/// - The returned string is a JSON array of extensions (must be freed with `kreuzberg_free_string`)
/// - Returns NULL on error (check `kreuzberg_last_error`)
///
/// # Example (C)
///
/// ```c
/// char* extensions = kreuzberg_get_extensions_for_mime("application/pdf");
/// if (extensions == NULL) {
///     const char* error = kreuzberg_last_error();
///     printf("Failed to get extensions: %s\n", error);
/// } else {
///     printf("Extensions: %s\n", extensions);
///     kreuzberg_free_string(extensions);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_get_extensions_for_mime(mime_type: *const c_char) -> *mut c_char {
    ffi_panic_guard!("kreuzberg_get_extensions_for_mime", {
        clear_last_error();

        if mime_type.is_null() {
            set_last_error("mime_type cannot be NULL".to_string());
            return ptr::null_mut();
        }

        let mime_str = match unsafe { CStr::from_ptr(mime_type) }.to_str() {
            Ok(s) => s,
            Err(e) => {
                set_last_error(format!("Invalid UTF-8 in MIME type: {}", e));
                return ptr::null_mut();
            }
        };

        match kreuzberg::core::mime::get_extensions_for_mime(mime_str) {
            Ok(extensions) => match serde_json::to_string(&extensions) {
                Ok(json) => match string_to_c_string(json) {
                    Ok(ptr) => ptr,
                    Err(e) => {
                        set_last_error(e);
                        ptr::null_mut()
                    }
                },
                Err(e) => {
                    set_last_error(format!("Failed to serialize extensions: {}", e));
                    ptr::null_mut()
                }
            },
            Err(e) => {
                set_last_error(e.to_string());
                ptr::null_mut()
            }
        }
    })
}

/// Load an ExtractionConfig from a file.
///
/// Automatically detects the file format based on extension:
/// - `.toml` - TOML format
/// - `.yaml`, `.yml` - YAML format
/// - `.json` - JSON format
///
/// # Safety
///
/// - `path` must be a valid null-terminated C string representing a file path
/// - Returns a pointer to ExtractionConfig on success, NULL on error
/// - The returned config must be freed with `kreuzberg_free_config`
/// - Check `kreuzberg_last_error` on NULL return
///
/// # Example (C)
///
/// ```c
/// ExtractionConfig* config = kreuzberg_config_from_file("kreuzberg.toml");
/// if (config == NULL) {
///     const char* error = kreuzberg_last_error();
///     printf("Failed to load config: %s\n", error);
///     return 1;
/// }
///
/// // Use config...
/// char* result = kreuzberg_extract_file_with_config_sync("document.pdf", config);
///
/// kreuzberg_free_config(config);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_config_from_file(path: *const c_char) -> *mut ExtractionConfig {
    ffi_panic_guard!("kreuzberg_config_from_file", {
        clear_last_error();

        if path.is_null() {
            set_last_error("Config path cannot be NULL".to_string());
            return ptr::null_mut();
        }

        let path_str = match unsafe { CStr::from_ptr(path) }.to_str() {
            Ok(s) => s,
            Err(e) => {
                set_last_error(format!("Invalid UTF-8 in config path: {}", e));
                return ptr::null_mut();
            }
        };

        let path_buf = Path::new(path_str);

        match ExtractionConfig::from_file(path_buf) {
            Ok(config) => Box::into_raw(Box::new(config)),
            Err(e) => {
                // ~keep: IO errors from file operations should bubble up as they indicate
                match &e {
                    KreuzbergError::Io(io_err) => {
                        set_last_error(format!("IO error loading config: {}", io_err));
                    }
                    _ => {
                        set_last_error(format!("Failed to load config from file: {}", e));
                    }
                }
                ptr::null_mut()
            }
        }
    })
}

/// Discover and load an ExtractionConfig by searching parent directories.
///
/// Searches the current directory and all parent directories for:
/// - `kreuzberg.toml`
/// - `kreuzberg.yaml`
/// - `kreuzberg.yml`
/// - `kreuzberg.json`
///
/// Returns the first config file found as JSON, or NULL if none found.
///
/// # Safety
///
/// - The returned string must be freed with `kreuzberg_free_string`
/// - Returns NULL if no config found or on error (check `kreuzberg_last_error`)
///
/// # Example (C)
///
/// ```c
/// char* config_json = kreuzberg_config_discover();
/// if (config_json == NULL) {
///     const char* error = kreuzberg_last_error();
///     if (error != NULL && strlen(error) > 0) {
///         printf("Error discovering config: %s\n", error);
///         return 1;
///     }
///     // No config found, use defaults
///     printf("No config file found\n");
/// } else {
///     printf("Config: %s\n", config_json);
///     kreuzberg_free_string(config_json);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kreuzberg_config_discover() -> *mut c_char {
    ffi_panic_guard!("kreuzberg_config_discover", {
        clear_last_error();

        match ExtractionConfig::discover() {
            Ok(Some(config)) => match serde_json::to_string(&config) {
                Ok(json) => match CString::new(json) {
                    Ok(cstr) => cstr.into_raw(),
                    Err(e) => {
                        set_last_error(format!("Failed to serialize config: {}", e));
                        ptr::null_mut()
                    }
                },
                Err(e) => {
                    set_last_error(format!("Failed to serialize config: {}", e));
                    ptr::null_mut()
                }
            },
            Ok(None) => ptr::null_mut(),
            Err(e) => {
                // ~keep: IO errors from directory traversal should bubble up as they indicate
                match &e {
                    KreuzbergError::Io(io_err) => {
                        set_last_error(format!("IO error discovering config: {}", io_err));
                    }
                    _ => {
                        set_last_error(format!("Failed to discover config: {}", e));
                    }
                }
                ptr::null_mut()
            }
        }
    })
}

// Static assertions to verify FFI struct sizes match Java MemoryLayout definitions.
// These assertions ensure that alignment and padding are correct for FFI interoperability.
//
// Expected sizes (on 64-bit systems):
// - CExtractionResult: 88 bytes (10 pointers + 1 bool + 7 bytes padding)
// - CBatchResult: 24 bytes (1 pointer + 1 usize + 1 bool + 7 bytes padding)
// - CBytesWithMime: 24 bytes (1 pointer + 1 usize + 1 pointer, naturally aligned)

#[allow(non_upper_case_globals)]
const _: () = {
    const fn assert_c_extraction_result_size() {
        const SIZE: usize = std::mem::size_of::<CExtractionResult>();
        const _: () = assert!(SIZE == 88, "CExtractionResult size must be 88 bytes");
    }

    const fn assert_c_extraction_result_alignment() {
        const ALIGN: usize = std::mem::align_of::<CExtractionResult>();
        const _: () = assert!(ALIGN == 8, "CExtractionResult alignment must be 8 bytes");
    }

    const fn assert_c_batch_result_size() {
        const SIZE: usize = std::mem::size_of::<CBatchResult>();
        const _: () = assert!(SIZE == 24, "CBatchResult size must be 24 bytes");
    }

    const fn assert_c_batch_result_alignment() {
        const ALIGN: usize = std::mem::align_of::<CBatchResult>();
        const _: () = assert!(ALIGN == 8, "CBatchResult alignment must be 8 bytes");
    }

    const fn assert_c_bytes_with_mime_size() {
        const SIZE: usize = std::mem::size_of::<CBytesWithMime>();
        const _: () = assert!(SIZE == 24, "CBytesWithMime size must be 24 bytes");
    }

    const fn assert_c_bytes_with_mime_alignment() {
        const ALIGN: usize = std::mem::align_of::<CBytesWithMime>();
        const _: () = assert!(ALIGN == 8, "CBytesWithMime alignment must be 8 bytes");
    }

    let _ = assert_c_extraction_result_size;
    let _ = assert_c_extraction_result_alignment;
    let _ = assert_c_batch_result_size;
    let _ = assert_c_batch_result_alignment;
    let _ = assert_c_bytes_with_mime_size;
    let _ = assert_c_bytes_with_mime_alignment;
};

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
