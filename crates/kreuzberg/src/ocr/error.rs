use std::fmt;

/// OCR-specific errors (pure Rust, no PyO3)
#[derive(Debug, Clone)]
pub enum OcrError {
    TesseractInitializationFailed(String),
    UnsupportedVersion(String),
    InvalidConfiguration(String),
    InvalidLanguageCode(String),
    ImageProcessingFailed(String),
    ProcessingFailed(String),
    CacheError(String),
    IOError(String),
}

impl fmt::Display for OcrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TesseractInitializationFailed(msg) => {
                write!(f, "Tesseract initialization failed: {}", msg)
            }
            Self::UnsupportedVersion(msg) => {
                write!(f, "Unsupported Tesseract version: {}", msg)
            }
            Self::InvalidConfiguration(msg) => write!(f, "Invalid configuration: {}", msg),
            Self::InvalidLanguageCode(msg) => write!(f, "Invalid language code: {}", msg),
            Self::ImageProcessingFailed(msg) => write!(f, "Image processing failed: {}", msg),
            Self::ProcessingFailed(msg) => write!(f, "OCR processing failed: {}", msg),
            Self::CacheError(msg) => write!(f, "Cache error: {}", msg),
            Self::IOError(msg) => write!(f, "I/O error: {}", msg),
        }
    }
}

impl std::error::Error for OcrError {}

impl From<std::io::Error> for OcrError {
    fn from(err: std::io::Error) -> Self {
        Self::IOError(err.to_string())
    }
}
