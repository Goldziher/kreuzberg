//! Error conversion from Rust to Python exceptions
//!
//! Converts `KreuzbergError` from the Rust core into appropriate Python exceptions.

use pyo3::{exceptions::*, prelude::*};

/// Format an error message with its source chain.
///
/// If the source is present, formats as "message: source".
/// Otherwise, returns just the message.
///
/// This preserves the full error context when converting Rust errors to Python.
fn format_error_with_source(message: String, source: Option<Box<dyn std::error::Error + Send + Sync>>) -> String {
    if let Some(src) = source {
        format!("{}: {}", message, src)
    } else {
        message
    }
}

/// Convert Rust KreuzbergError to Python exception.
///
/// Maps error variants to appropriate Python exception types:
/// - `Validation` → `ValueError`
/// - `UnsupportedFormat` → `ValueError`
/// - `Parsing` → `RuntimeError`
/// - `Io` → `IOError`
/// - `Ocr` → `RuntimeError`
/// - `Plugin` → `RuntimeError`
/// - `LockPoisoned` → `RuntimeError`
/// - `Cache` → `RuntimeError`
/// - `ImageProcessing` → `RuntimeError`
/// - `Serialization` → `RuntimeError`
/// - `MissingDependency` → `RuntimeError`
/// - `Other` → `RuntimeError`
///
/// All errors preserve their source chain for better debugging.
pub fn to_py_err(error: kreuzberg::KreuzbergError) -> PyErr {
    use kreuzberg::KreuzbergError;

    match error {
        KreuzbergError::Validation { message, source } => {
            PyValueError::new_err(format_error_with_source(message, source))
        }
        KreuzbergError::UnsupportedFormat(msg) => PyValueError::new_err(msg),
        KreuzbergError::Parsing { message, source } => {
            PyRuntimeError::new_err(format_error_with_source(message, source))
        }
        KreuzbergError::Io(e) => PyIOError::new_err(e.to_string()),
        KreuzbergError::Ocr { message, source } => PyRuntimeError::new_err(format_error_with_source(message, source)),
        KreuzbergError::Plugin { message, plugin_name } => {
            PyRuntimeError::new_err(format!("Plugin error in '{}': {}", plugin_name, message))
        }
        KreuzbergError::LockPoisoned(msg) => PyRuntimeError::new_err(format!("Lock poisoned: {}", msg)),
        KreuzbergError::Cache { message, source } => PyRuntimeError::new_err(format_error_with_source(message, source)),
        KreuzbergError::ImageProcessing { message, source } => {
            PyRuntimeError::new_err(format_error_with_source(message, source))
        }
        KreuzbergError::Serialization { message, source } => {
            PyRuntimeError::new_err(format_error_with_source(message, source))
        }
        KreuzbergError::MissingDependency(msg) => PyRuntimeError::new_err(msg),
        KreuzbergError::Other(msg) => PyRuntimeError::new_err(msg),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kreuzberg::KreuzbergError;

    #[test]
    fn test_validation_error_with_source() {
        Python::with_gil(|_py| {
            let source = std::io::Error::new(std::io::ErrorKind::InvalidInput, "bad input");
            let error = KreuzbergError::validation_with_source("Invalid configuration", source);
            let py_err = to_py_err(error);

            let err_msg = format!("{}", py_err);
            assert!(err_msg.contains("Invalid configuration"));
            assert!(err_msg.contains("bad input"));
            assert!(err_msg.contains("ValueError"));
        });
    }

    #[test]
    fn test_validation_error_without_source() {
        Python::with_gil(|_py| {
            let error = KreuzbergError::validation("Invalid configuration");
            let py_err = to_py_err(error);

            let err_msg = format!("{}", py_err);
            assert!(err_msg.contains("Invalid configuration"));
            assert!(err_msg.contains("ValueError"));
        });
    }

    #[test]
    fn test_parsing_error_with_source() {
        Python::with_gil(|_py| {
            let source = std::io::Error::new(std::io::ErrorKind::InvalidData, "corrupt file");
            let error = KreuzbergError::parsing_with_source("Failed to parse PDF", source);
            let py_err = to_py_err(error);

            let err_msg = format!("{}", py_err);
            assert!(err_msg.contains("Failed to parse PDF"));
            assert!(err_msg.contains("corrupt file"));
            assert!(err_msg.contains("RuntimeError"));
        });
    }

    #[test]
    fn test_parsing_error_without_source() {
        Python::with_gil(|_py| {
            let error = KreuzbergError::parsing("Failed to parse PDF");
            let py_err = to_py_err(error);

            let err_msg = format!("{}", py_err);
            assert!(err_msg.contains("Failed to parse PDF"));
            assert!(err_msg.contains("RuntimeError"));
        });
    }

    #[test]
    fn test_ocr_error_with_source() {
        Python::with_gil(|_py| {
            let source = std::io::Error::other("tesseract failed");
            let error = KreuzbergError::ocr_with_source("OCR processing failed", source);
            let py_err = to_py_err(error);

            let err_msg = format!("{}", py_err);
            assert!(err_msg.contains("OCR processing failed"));
            assert!(err_msg.contains("tesseract failed"));
            assert!(err_msg.contains("RuntimeError"));
        });
    }

    #[test]
    fn test_ocr_error_without_source() {
        Python::with_gil(|_py| {
            let error = KreuzbergError::ocr("OCR processing failed");
            let py_err = to_py_err(error);

            let err_msg = format!("{}", py_err);
            assert!(err_msg.contains("OCR processing failed"));
            assert!(err_msg.contains("RuntimeError"));
        });
    }

    #[test]
    fn test_plugin_error() {
        Python::with_gil(|_py| {
            let error = KreuzbergError::Plugin {
                message: "Extraction failed".to_string(),
                plugin_name: "pdf-extractor".to_string(),
            };
            let py_err = to_py_err(error);

            let err_msg = format!("{}", py_err);
            assert!(err_msg.contains("Plugin error in 'pdf-extractor'"));
            assert!(err_msg.contains("Extraction failed"));
            assert!(err_msg.contains("RuntimeError"));
        });
    }

    #[test]
    fn test_cache_error_with_source() {
        Python::with_gil(|_py| {
            let source = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "permission denied");
            let error = KreuzbergError::Cache {
                message: "Cache write failed".to_string(),
                source: Some(Box::new(source)),
            };
            let py_err = to_py_err(error);

            let err_msg = format!("{}", py_err);
            assert!(err_msg.contains("Cache write failed"));
            assert!(err_msg.contains("permission denied"));
            assert!(err_msg.contains("RuntimeError"));
        });
    }

    #[test]
    fn test_image_processing_error_with_source() {
        Python::with_gil(|_py| {
            let source = std::io::Error::other("resize failed");
            let error = KreuzbergError::ImageProcessing {
                message: "Image processing failed".to_string(),
                source: Some(Box::new(source)),
            };
            let py_err = to_py_err(error);

            let err_msg = format!("{}", py_err);
            assert!(err_msg.contains("Image processing failed"));
            assert!(err_msg.contains("resize failed"));
            assert!(err_msg.contains("RuntimeError"));
        });
    }

    #[test]
    fn test_serialization_error_with_source() {
        Python::with_gil(|_py| {
            let json_err = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
            let error: KreuzbergError = json_err.into();
            let py_err = to_py_err(error);

            let err_msg = format!("{}", py_err);
            assert!(err_msg.contains("Serialization error"));
            assert!(err_msg.contains("RuntimeError"));
        });
    }

    #[test]
    fn test_io_error() {
        Python::with_gil(|_py| {
            let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
            let error: KreuzbergError = io_err.into();
            let py_err = to_py_err(error);

            let err_msg = format!("{}", py_err);
            assert!(err_msg.contains("file not found"));
            assert!(err_msg.contains("IOError") || err_msg.contains("OSError"));
        });
    }

    #[test]
    fn test_unsupported_format_error() {
        Python::with_gil(|_py| {
            let error = KreuzbergError::UnsupportedFormat("application/unknown".to_string());
            let py_err = to_py_err(error);

            let err_msg = format!("{}", py_err);
            assert!(err_msg.contains("application/unknown"));
            assert!(err_msg.contains("ValueError"));
        });
    }

    #[test]
    fn test_missing_dependency_error() {
        Python::with_gil(|_py| {
            let error = KreuzbergError::MissingDependency("tesseract not found".to_string());
            let py_err = to_py_err(error);

            let err_msg = format!("{}", py_err);
            assert!(err_msg.contains("tesseract not found"));
            assert!(err_msg.contains("RuntimeError"));
        });
    }

    #[test]
    fn test_other_error() {
        Python::with_gil(|_py| {
            let error = KreuzbergError::Other("unexpected error".to_string());
            let py_err = to_py_err(error);

            let err_msg = format!("{}", py_err);
            assert!(err_msg.contains("unexpected error"));
            assert!(err_msg.contains("RuntimeError"));
        });
    }

    #[test]
    fn test_format_error_with_source_helper() {
        let source = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let formatted = format_error_with_source("Failed to open file".to_string(), Some(Box::new(source)));
        assert_eq!(formatted, "Failed to open file: file not found");
    }

    #[test]
    fn test_format_error_without_source_helper() {
        let formatted = format_error_with_source("Failed to open file".to_string(), None);
        assert_eq!(formatted, "Failed to open file");
    }

    #[test]
    fn test_lock_poisoned_error() {
        Python::with_gil(|_py| {
            let error = KreuzbergError::LockPoisoned("Registry lock poisoned".to_string());
            let py_err = to_py_err(error);

            let err_msg = format!("{}", py_err);
            assert!(err_msg.contains("Lock poisoned: Registry lock poisoned"));
            assert!(err_msg.contains("RuntimeError"));
        });
    }
}
