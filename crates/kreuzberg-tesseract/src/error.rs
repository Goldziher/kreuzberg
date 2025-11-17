use std::str::Utf8Error;
use thiserror::Error;

/// Errors that can occur when using the Tesseract API.
#[derive(Error, Debug)]
pub enum TesseractError {
    #[error("Failed to initialize Tesseract")]
    InitError,
    #[error("Failed to set image")]
    SetImageError,
    #[error("OCR operation failed")]
    OcrError,
    #[error("Invalid UTF-8 in Tesseract output")]
    Utf8Error(#[from] Utf8Error),
    #[error("Failed to lock mutex")]
    MutexLockError,
    #[error("Failed to set variable")]
    SetVariableError,
    #[error("Failed to get variable")]
    GetVariableError,
    #[error("Null pointer error")]
    NullPointerError,
    #[error("Invalid parameter")]
    InvalidParameterError,
    #[error("Layout analysis failed")]
    AnalyseLayoutError,
    #[error("Page processing failed")]
    ProcessPagesError,
    #[error("I/O error")]
    IoError,
    #[error("Mutex error")]
    MutexError,
    #[error("Invalid dimensions")]
    InvalidDimensions,
    #[error("Invalid bytes per pixel")]
    InvalidBytesPerPixel,
    #[error("Invalid bytes per line")]
    InvalidBytesPerLine,
    #[error("Invalid image data")]
    InvalidImageData,
    #[error("Uninitialized error")]
    UninitializedError,
}

/// Result type for Tesseract operations.
pub type Result<T> = std::result::Result<T, TesseractError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let error = TesseractError::InitError;
        assert_eq!(error.to_string(), "Failed to initialize Tesseract");

        let error = TesseractError::SetImageError;
        assert_eq!(error.to_string(), "Failed to set image");

        let error = TesseractError::OcrError;
        assert_eq!(error.to_string(), "OCR operation failed");
    }

    #[test]
    fn test_utf8_error_conversion() {
        let invalid_utf8 = vec![0xFF, 0xFE];
        let utf8_error = std::str::from_utf8(&invalid_utf8).unwrap_err();
        let tess_error: TesseractError = utf8_error.into();

        match tess_error {
            TesseractError::Utf8Error(_) => {}
            _ => panic!("Expected Utf8Error variant"),
        }
    }

    #[test]
    fn test_error_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<TesseractError>();
    }
}
