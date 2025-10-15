use thiserror::Error;

pub type Result<T> = std::result::Result<T, KreuzbergError>;

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

    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),

    #[error("{0}")]
    Other(String),
}

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

impl From<crate::pdf::error::PdfError> for KreuzbergError {
    fn from(err: crate::pdf::error::PdfError) -> Self {
        KreuzbergError::Parsing {
            message: err.to_string(),
            source: Some(Box::new(err)),
        }
    }
}

impl KreuzbergError {
    /// Create a Parsing error without a source
    pub fn parsing<S: Into<String>>(message: S) -> Self {
        Self::Parsing {
            message: message.into(),
            source: None,
        }
    }

    /// Create a Parsing error with a source
    pub fn parsing_with_source<S: Into<String>, E: std::error::Error + Send + Sync + 'static>(
        message: S,
        source: E,
    ) -> Self {
        Self::Parsing {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create an OCR error without a source
    pub fn ocr<S: Into<String>>(message: S) -> Self {
        Self::Ocr {
            message: message.into(),
            source: None,
        }
    }

    /// Create an OCR error with a source
    pub fn ocr_with_source<S: Into<String>, E: std::error::Error + Send + Sync + 'static>(
        message: S,
        source: E,
    ) -> Self {
        Self::Ocr {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a Validation error without a source
    pub fn validation<S: Into<String>>(message: S) -> Self {
        Self::Validation {
            message: message.into(),
            source: None,
        }
    }

    /// Create a Validation error with a source
    pub fn validation_with_source<S: Into<String>, E: std::error::Error + Send + Sync + 'static>(
        message: S,
        source: E,
    ) -> Self {
        Self::Validation {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a Cache error without a source
    pub fn cache<S: Into<String>>(message: S) -> Self {
        Self::Cache {
            message: message.into(),
            source: None,
        }
    }

    /// Create an ImageProcessing error without a source
    pub fn image_processing<S: Into<String>>(message: S) -> Self {
        Self::ImageProcessing {
            message: message.into(),
            source: None,
        }
    }

    /// Create a Serialization error without a source
    pub fn serialization<S: Into<String>>(message: S) -> Self {
        Self::Serialization {
            message: message.into(),
            source: None,
        }
    }
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
    fn test_image_processing_error() {
        let err = KreuzbergError::image_processing("resize failed");
        assert_eq!(err.to_string(), "Image processing error: resize failed");
    }

    #[test]
    fn test_serialization_error() {
        let err = KreuzbergError::serialization("JSON parse error");
        assert_eq!(err.to_string(), "Serialization error: JSON parse error");
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
        // Test encoding error by using invalid message pack data
        use std::collections::HashMap;
        let mut map: HashMap<Vec<u8>, String> = HashMap::new();
        // Binary keys are not supported in MessagePack
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
}
