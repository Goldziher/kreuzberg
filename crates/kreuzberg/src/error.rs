//! Error types for Kreuzberg.
//!
//! This module defines all error types used throughout the library. All errors
//! inherit from `KreuzbergError` and follow Rust error handling best practices:
//!
//! - Use `thiserror` for automatic `Error` trait implementation
//! - Preserve error chains with `#[source]` attributes
//! - Include context in error messages (file paths, config values, etc.)
//!
//! # Error Handling Philosophy
//!
//! **System errors MUST always bubble up unchanged:**
//! - `KreuzbergError::Io` (from `std::io::Error`) - File system errors, permission errors
//! - These indicate real system problems that users need to know about
//! - Never wrap or suppress these - they must surface to enable bug reports
//!
//! **Application errors are wrapped with context:**
//! - `Parsing` - Document format errors, corrupt files
//! - `Validation` - Invalid configuration or parameters
//! - `Ocr` - OCR processing failures
//! - `MissingDependency` - Missing optional system dependencies
//!
//! # Example
//!
//! ```rust
//! use kreuzberg::{KreuzbergError, Result};
//!
//! fn process_file(path: &str) -> Result<String> {
//!     // IO errors bubble up automatically via ?
//!     let content = std::fs::read_to_string(path)?;
//!
//!     // Application errors include context
//!     if content.is_empty() {
//!         return Err(KreuzbergError::validation(
//!             format!("File is empty: {}", path)
//!         ));
//!     }
//!
//!     Ok(content)
//! }
//! ```
use thiserror::Error;

/// Result type alias using `KreuzbergError`.
///
/// This is the standard return type for all fallible operations in Kreuzberg.
pub type Result<T> = std::result::Result<T, KreuzbergError>;

/// Main error type for all Kreuzberg operations.
///
/// All errors in Kreuzberg use this enum, which preserves error chains
/// and provides context for debugging.
///
/// # Variants
///
/// - `Io` - File system and I/O errors (always bubble up)
/// - `Parsing` - Document parsing errors (corrupt files, unsupported features)
/// - `Ocr` - OCR processing errors
/// - `Validation` - Input validation errors (invalid paths, config, parameters)
/// - `Cache` - Cache operation errors (non-fatal, can be ignored)
/// - `ImageProcessing` - Image manipulation errors
/// - `Serialization` - JSON/MessagePack serialization errors
/// - `MissingDependency` - Missing optional dependencies (tesseract, pandoc, etc.)
/// - `Plugin` - Plugin-specific errors
/// - `LockPoisoned` - Mutex/RwLock poisoning (should not happen in normal operation)
/// - `UnsupportedFormat` - Unsupported MIME type or file format
/// - `Other` - Catch-all for uncommon errors
#[derive(Debug, Error)]
pub enum KreuzbergError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parsing error: {message}")]
    Parsing {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("OCR error: {message}")]
    Ocr {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("Validation error: {message}")]
    Validation {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("Cache error: {message}")]
    Cache {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("Image processing error: {message}")]
    ImageProcessing {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("Serialization error: {message}")]
    Serialization {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("Missing dependency: {0}")]
    MissingDependency(String),

    #[error("Plugin error in '{plugin_name}': {message}")]
    Plugin { message: String, plugin_name: String },

    #[error("Lock poisoned: {0}")]
    LockPoisoned(String),

    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),

    #[error("{0}")]
    Other(String),
}

#[cfg(feature = "excel")]
impl From<calamine::Error> for KreuzbergError {
    fn from(err: calamine::Error) -> Self {
        KreuzbergError::Parsing {
            message: err.to_string(),
            source: Some(Box::new(err)),
        }
    }
}

impl From<serde_json::Error> for KreuzbergError {
    fn from(err: serde_json::Error) -> Self {
        KreuzbergError::Serialization {
            message: err.to_string(),
            source: Some(Box::new(err)),
        }
    }
}

impl From<rmp_serde::encode::Error> for KreuzbergError {
    fn from(err: rmp_serde::encode::Error) -> Self {
        KreuzbergError::Serialization {
            message: err.to_string(),
            source: Some(Box::new(err)),
        }
    }
}

impl From<rmp_serde::decode::Error> for KreuzbergError {
    fn from(err: rmp_serde::decode::Error) -> Self {
        KreuzbergError::Serialization {
            message: err.to_string(),
            source: Some(Box::new(err)),
        }
    }
}

#[cfg(feature = "pdf")]
impl From<crate::pdf::error::PdfError> for KreuzbergError {
    fn from(err: crate::pdf::error::PdfError) -> Self {
        KreuzbergError::Parsing {
            message: err.to_string(),
            source: Some(Box::new(err)),
        }
    }
}

macro_rules! error_constructor {
    ($name:ident, $variant:ident) => {
        paste::paste! {
            #[doc = "Create a " $variant " error"]
            pub fn $name<S: Into<String>>(message: S) -> Self {
                Self::$variant {
                    message: message.into(),
                    source: None,
                }
            }

            #[doc = "Create a " $variant " error with source"]
            pub fn [<$name _with_source>]<S: Into<String>, E: std::error::Error + Send + Sync + 'static>(
                message: S,
                source: E,
            ) -> Self {
                Self::$variant {
                    message: message.into(),
                    source: Some(Box::new(source)),
                }
            }
        }
    };
}

impl KreuzbergError {
    error_constructor!(parsing, Parsing);
    error_constructor!(ocr, Ocr);
    error_constructor!(validation, Validation);
    error_constructor!(cache, Cache);
    error_constructor!(image_processing, ImageProcessing);
    error_constructor!(serialization, Serialization);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_io_error_from() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let krz_err: KreuzbergError = io_err.into();
        assert!(matches!(krz_err, KreuzbergError::Io(_)));
        assert!(krz_err.to_string().contains("IO error"));
    }

    #[test]
    fn test_parsing_error() {
        let err = KreuzbergError::parsing("invalid format");
        assert_eq!(err.to_string(), "Parsing error: invalid format");
    }

    #[test]
    fn test_parsing_error_with_source() {
        let source = std::io::Error::new(std::io::ErrorKind::InvalidData, "bad data");
        let err = KreuzbergError::parsing_with_source("invalid format", source);
        assert_eq!(err.to_string(), "Parsing error: invalid format");
        assert!(std::error::Error::source(&err).is_some());
    }

    #[test]
    fn test_ocr_error() {
        let err = KreuzbergError::ocr("OCR failed");
        assert_eq!(err.to_string(), "OCR error: OCR failed");
    }

    #[test]
    fn test_ocr_error_with_source() {
        let source = std::io::Error::other("tesseract failed");
        let err = KreuzbergError::ocr_with_source("OCR failed", source);
        assert_eq!(err.to_string(), "OCR error: OCR failed");
        assert!(std::error::Error::source(&err).is_some());
    }

    #[test]
    fn test_validation_error() {
        let err = KreuzbergError::validation("invalid input");
        assert_eq!(err.to_string(), "Validation error: invalid input");
    }

    #[test]
    fn test_validation_error_with_source() {
        let source = std::io::Error::new(std::io::ErrorKind::InvalidInput, "bad param");
        let err = KreuzbergError::validation_with_source("invalid input", source);
        assert_eq!(err.to_string(), "Validation error: invalid input");
        assert!(std::error::Error::source(&err).is_some());
    }

    #[test]
    fn test_cache_error() {
        let err = KreuzbergError::cache("cache write failed");
        assert_eq!(err.to_string(), "Cache error: cache write failed");
    }

    #[test]
    fn test_cache_error_with_source() {
        let source = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "cannot write");
        let err = KreuzbergError::cache_with_source("cache write failed", source);
        assert_eq!(err.to_string(), "Cache error: cache write failed");
        assert!(std::error::Error::source(&err).is_some());
    }

    #[test]
    fn test_image_processing_error() {
        let err = KreuzbergError::image_processing("resize failed");
        assert_eq!(err.to_string(), "Image processing error: resize failed");
    }

    #[test]
    fn test_image_processing_error_with_source() {
        let source = std::io::Error::other("image decode failed");
        let err = KreuzbergError::image_processing_with_source("resize failed", source);
        assert_eq!(err.to_string(), "Image processing error: resize failed");
        assert!(std::error::Error::source(&err).is_some());
    }

    #[test]
    fn test_serialization_error() {
        let err = KreuzbergError::serialization("JSON parse error");
        assert_eq!(err.to_string(), "Serialization error: JSON parse error");
    }

    #[test]
    fn test_serialization_error_with_source() {
        let source = std::io::Error::new(std::io::ErrorKind::InvalidData, "bad format");
        let err = KreuzbergError::serialization_with_source("JSON parse error", source);
        assert_eq!(err.to_string(), "Serialization error: JSON parse error");
        assert!(std::error::Error::source(&err).is_some());
    }

    #[test]
    fn test_missing_dependency_error() {
        let err = KreuzbergError::MissingDependency("tesseract not found".to_string());
        assert_eq!(err.to_string(), "Missing dependency: tesseract not found");
    }

    #[test]
    fn test_plugin_error() {
        let err = KreuzbergError::Plugin {
            message: "extraction failed".to_string(),
            plugin_name: "pdf-extractor".to_string(),
        };
        assert_eq!(err.to_string(), "Plugin error in 'pdf-extractor': extraction failed");
    }

    #[test]
    fn test_unsupported_format_error() {
        let err = KreuzbergError::UnsupportedFormat("application/unknown".to_string());
        assert_eq!(err.to_string(), "Unsupported format: application/unknown");
    }

    #[test]
    fn test_other_error() {
        let err = KreuzbergError::Other("unexpected error".to_string());
        assert_eq!(err.to_string(), "unexpected error");
    }

    #[test]
    #[cfg(feature = "excel")]
    fn test_calamine_error_conversion() {
        let cal_err = calamine::Error::Msg("invalid Excel file");
        let krz_err: KreuzbergError = cal_err.into();
        assert!(matches!(krz_err, KreuzbergError::Parsing { .. }));
        assert!(krz_err.to_string().contains("Parsing error"));
    }

    #[test]
    fn test_serde_json_error_conversion() {
        let json_err = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
        let krz_err: KreuzbergError = json_err.into();
        assert!(matches!(krz_err, KreuzbergError::Serialization { .. }));
        assert!(krz_err.to_string().contains("Serialization error"));
    }

    #[test]
    fn test_rmp_encode_error_conversion() {
        use std::collections::HashMap;
        let mut map: HashMap<Vec<u8>, String> = HashMap::new();
        map.insert(vec![255, 255], "test".to_string());

        let result = rmp_serde::to_vec(&map);
        if let Err(rmp_err) = result {
            let krz_err: KreuzbergError = rmp_err.into();
            assert!(matches!(krz_err, KreuzbergError::Serialization { .. }));
        }
    }

    #[test]
    fn test_rmp_decode_error_conversion() {
        let invalid_msgpack = vec![0xFF, 0xFF, 0xFF];
        let rmp_err = rmp_serde::from_slice::<String>(&invalid_msgpack).unwrap_err();
        let krz_err: KreuzbergError = rmp_err.into();
        assert!(matches!(krz_err, KreuzbergError::Serialization { .. }));
    }

    #[test]
    #[cfg(feature = "pdf")]
    fn test_pdf_error_conversion() {
        let pdf_err = crate::pdf::error::PdfError::InvalidPdf("corrupt PDF".to_string());
        let krz_err: KreuzbergError = pdf_err.into();
        assert!(matches!(krz_err, KreuzbergError::Parsing { .. }));
    }

    #[test]
    fn test_error_debug() {
        let err = KreuzbergError::validation("test");
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("Validation"));
    }

    #[test]
    fn test_lock_poisoned_error() {
        let err = KreuzbergError::LockPoisoned("Registry lock poisoned".to_string());
        assert_eq!(err.to_string(), "Lock poisoned: Registry lock poisoned");
    }

    #[test]
    fn test_io_error_bubbles_unchanged() {
        // Test that io::Error converts to KreuzbergError::Io via ? operator
        fn read_file() -> Result<String> {
            let content = std::fs::read_to_string("/nonexistent/file.txt")?;
            Ok(content)
        }

        let result = read_file();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KreuzbergError::Io(_)));
    }

    #[test]
    fn test_io_error_not_found() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let krz_err: KreuzbergError = io_err.into();
        assert!(matches!(krz_err, KreuzbergError::Io(_)));
        assert!(krz_err.to_string().contains("file not found"));
    }

    #[test]
    fn test_io_error_permission_denied() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "permission denied");
        let krz_err: KreuzbergError = io_err.into();
        assert!(matches!(krz_err, KreuzbergError::Io(_)));
        assert!(krz_err.to_string().contains("permission denied"));
    }

    #[test]
    fn test_io_error_invalid_data_vs_parsing() {
        // InvalidData from io::Error should still be Io variant
        let io_err = std::io::Error::new(std::io::ErrorKind::InvalidData, "corrupted data");
        let krz_err: KreuzbergError = io_err.into();
        assert!(matches!(krz_err, KreuzbergError::Io(_)));

        // But parsing errors are different
        let parse_err = KreuzbergError::parsing("corrupted format");
        assert!(matches!(parse_err, KreuzbergError::Parsing { .. }));
    }
}
