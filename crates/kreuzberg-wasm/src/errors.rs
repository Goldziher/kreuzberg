//! Error handling for WASM bindings
//!
//! This module defines error types and handling mechanisms for WASM environments.
//! Converts Kreuzberg errors to JsValue for JavaScript/TypeScript consumers.

use kreuzberg::KreuzbergError;
use wasm_bindgen::prelude::*;

/// Converts KreuzbergError to JsValue with detailed error information.
///
/// Maps Kreuzberg error variants to JavaScript error objects with appropriate
/// error types and messages:
///
/// - `Io` → Generic I/O error
/// - `Parsing` → Parsing/malformed document error
/// - `Ocr` → OCR processing error
/// - `Validation` → Invalid configuration or parameters
/// - `Cache` → Cache-related error
/// - `ImageProcessing` → Image manipulation error
/// - `Serialization` → JSON/serialization error
/// - `MissingDependency` → Missing system dependency
/// - `Plugin` → Plugin-specific error
/// - `LockPoisoned` → Lock poisoning (internal error)
/// - `UnsupportedFormat` → Unsupported MIME type
/// - `Other` → Generic error
pub fn convert_error(err: KreuzbergError) -> JsValue {
    use kreuzberg::KreuzbergError;

    let (error_type, message) = match err {
        KreuzbergError::Io(e) => ("IOError", format!("IO error: {}", e)),

        KreuzbergError::Parsing { message, .. } => ("ParsingError", format!("Parsing error: {}", message)),

        KreuzbergError::Ocr { message, .. } => ("OCRError", format!("OCR error: {}", message)),

        KreuzbergError::Validation { message, .. } => ("ValidationError", format!("Validation error: {}", message)),

        KreuzbergError::Cache { message, .. } => ("CacheError", format!("Cache error: {}", message)),

        KreuzbergError::ImageProcessing { message, .. } => {
            ("ImageProcessingError", format!("Image processing error: {}", message))
        }

        KreuzbergError::Serialization { message, .. } => {
            ("SerializationError", format!("Serialization error: {}", message))
        }

        KreuzbergError::MissingDependency(dep) => ("MissingDependencyError", format!("Missing dependency: {}", dep)),

        KreuzbergError::Plugin { message, plugin_name } => {
            ("PluginError", format!("Plugin error in '{}': {}", plugin_name, message))
        }

        KreuzbergError::LockPoisoned(msg) => ("LockPoisonedError", format!("Lock poisoned: {}", msg)),

        KreuzbergError::UnsupportedFormat(format) => {
            ("UnsupportedFormatError", format!("Unsupported format: {}", format))
        }

        KreuzbergError::Other(msg) => ("Error", msg),
    };

    // Create a JavaScript error object
    let error_constructor = js_sys::Reflect::get(&js_sys::global(), &JsValue::from_str("Error"))
        .ok()
        .and_then(|f| {
            if f.is_function() {
                Some(f.unchecked_into::<js_sys::Function>())
            } else {
                None
            }
        });

    match error_constructor {
        Some(ctor) => js_sys::Reflect::construct(&ctor, &js_sys::Array::of1(&JsValue::from_str(&message)))
            .unwrap_or_else(|_| JsValue::from_str(&format!("{}: {}", error_type, message))),
        None => JsValue::from_str(&format!("{}: {}", error_type, message)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_convert_error_io_error_returns_jsvalue() {
        let err = KreuzbergError::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "file not found"));
        let result = convert_error(err);

        assert!(!result.is_null());
        assert!(!result.is_undefined());
    }

    #[wasm_bindgen_test]
    fn test_convert_error_parsing_error_returns_jsvalue() {
        let err = KreuzbergError::Parsing {
            message: "Invalid PDF header".to_string(),
            source: None,
        };
        let result = convert_error(err);

        assert!(!result.is_null());
    }

    #[wasm_bindgen_test]
    fn test_convert_error_ocr_error_returns_jsvalue() {
        let err = KreuzbergError::Ocr {
            message: "OCR processing failed".to_string(),
            source: None,
        };
        let result = convert_error(err);

        assert!(!result.is_null());
    }

    #[wasm_bindgen_test]
    fn test_convert_error_validation_error_returns_jsvalue() {
        let err = KreuzbergError::Validation {
            message: "Invalid configuration".to_string(),
            source: None,
        };
        let result = convert_error(err);

        assert!(!result.is_null());
    }

    #[wasm_bindgen_test]
    fn test_convert_error_cache_error_returns_jsvalue() {
        let err = KreuzbergError::Cache {
            message: "Cache write failed".to_string(),
            source: None,
        };
        let result = convert_error(err);

        assert!(!result.is_null());
    }

    #[wasm_bindgen_test]
    fn test_convert_error_image_processing_error_returns_jsvalue() {
        let err = KreuzbergError::ImageProcessing {
            message: "Image resize failed".to_string(),
            source: None,
        };
        let result = convert_error(err);

        assert!(!result.is_null());
    }

    #[wasm_bindgen_test]
    fn test_convert_error_serialization_error_returns_jsvalue() {
        let err = KreuzbergError::Serialization {
            message: "JSON serialization failed".to_string(),
            source: None,
        };
        let result = convert_error(err);

        assert!(!result.is_null());
    }

    #[wasm_bindgen_test]
    fn test_convert_error_missing_dependency_returns_jsvalue() {
        let err = KreuzbergError::MissingDependency("libreoffice".to_string());
        let result = convert_error(err);

        assert!(!result.is_null());
    }

    #[wasm_bindgen_test]
    fn test_convert_error_plugin_error_returns_jsvalue() {
        let err = KreuzbergError::Plugin {
            message: "Plugin initialization failed".to_string(),
            plugin_name: "custom-processor".to_string(),
        };
        let result = convert_error(err);

        assert!(!result.is_null());
    }

    #[wasm_bindgen_test]
    fn test_convert_error_lock_poisoned_returns_jsvalue() {
        let err = KreuzbergError::LockPoisoned("registry".to_string());
        let result = convert_error(err);

        assert!(!result.is_null());
    }

    #[wasm_bindgen_test]
    fn test_convert_error_unsupported_format_returns_jsvalue() {
        let err = KreuzbergError::UnsupportedFormat("application/x-custom".to_string());
        let result = convert_error(err);

        assert!(!result.is_null());
    }

    #[wasm_bindgen_test]
    fn test_convert_error_other_returns_jsvalue() {
        let err = KreuzbergError::Other("Unknown error occurred".to_string());
        let result = convert_error(err);

        assert!(!result.is_null());
    }
}
