use pyo3::PyErr;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum OCRError {
    #[error("Tesseract initialization failed: {0}")]
    InitializationFailed(String),

    #[error("Tesseract version {version} is not supported. Minimum version is {min_version}")]
    UnsupportedVersion { version: String, min_version: u32 },

    #[error("Language code '{code}' is not supported by Tesseract")]
    UnsupportedLanguage { code: String },

    #[error("Invalid language code format: {0}")]
    InvalidLanguageFormat(String),

    #[error("OCR processing failed: {0}")]
    ProcessingFailed(String),

    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    #[error("Image format not supported: {0}")]
    UnsupportedImageFormat(String),

    #[error("Failed to read image: {0}")]
    ImageReadError(String),

    #[error("Cache error: {0}")]
    CacheError(String),

    #[error("Internal error: {0}")]
    InternalError(String),
}

impl From<OCRError> for PyErr {
    fn from(err: OCRError) -> PyErr {
        pyo3::exceptions::PyRuntimeError::new_err(err.to_string())
    }
}
