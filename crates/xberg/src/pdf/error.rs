use std::fmt;

/// PDF-specific errors produced by the pure-Rust PDF backend.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone)]
pub enum PdfError {
    /// The file is not a valid PDF or the header is corrupted.
    InvalidPdf(String),
    /// The PDF is encrypted and no password was provided.
    PasswordRequired,
    /// The provided password did not decrypt the document.
    InvalidPassword,
    /// The encryption scheme is not supported by the pure-Rust backend.
    EncryptionNotSupported(String),
    /// The requested page index does not exist in the document.
    PageNotFound(usize),
    /// Extracting the text layer from a page failed.
    TextExtractionFailed(String),
    /// Page rendering to an image failed.
    RenderingFailed(String),
    /// Reading or parsing the PDF document info dictionary failed.
    MetadataExtractionFailed(String),
    /// A general extraction failure not covered by a more specific variant.
    ExtractionFailed(String),
    /// A required font could not be loaded or decoded.
    FontLoadingFailed(String),
    /// An I/O error occurred while reading the PDF stream.
    IOError(String),
    /// The operation was cancelled (reachable only via the REST async-jobs API).
    Cancelled,
}

impl fmt::Display for PdfError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PdfError::InvalidPdf(msg) => write!(f, "Invalid PDF: {}", msg),
            PdfError::PasswordRequired => write!(f, "PDF is password-protected"),
            PdfError::InvalidPassword => write!(f, "Invalid password provided"),
            PdfError::EncryptionNotSupported(msg) => {
                write!(f, "Encryption not supported: {}", msg)
            }
            PdfError::PageNotFound(page) => write!(f, "Page {} not found", page),
            PdfError::TextExtractionFailed(msg) => write!(f, "Text extraction failed: {}", msg),
            PdfError::RenderingFailed(msg) => write!(f, "Page rendering failed: {}", msg),
            PdfError::MetadataExtractionFailed(msg) => {
                write!(f, "Metadata extraction failed: {}", msg)
            }
            PdfError::ExtractionFailed(msg) => write!(f, "Extraction failed: {}", msg),
            PdfError::FontLoadingFailed(msg) => write!(f, "Font loading failed: {}", msg),
            PdfError::IOError(msg) => write!(f, "I/O error: {}", msg),
            PdfError::Cancelled => write!(f, "Extraction cancelled"),
        }
    }
}

impl std::error::Error for PdfError {}

// NOTE: No From<std::io::Error> impl - IO errors must bubble up unchanged per error handling policy

impl From<lopdf::Error> for PdfError {
    fn from(err: lopdf::Error) -> Self {
        match err {
            lopdf::Error::IO(io_err) => PdfError::IOError(io_err.to_string()),
            _ => PdfError::InvalidPdf(err.to_string()),
        }
    }
}

// Preserve the conversion accepted by 1.3.10 while extraction uses the current lopdf release. ~keep
impl From<lopdf_compat::Error> for PdfError {
    fn from(err: lopdf_compat::Error) -> Self {
        match err {
            lopdf_compat::Error::IO(io_err) => PdfError::IOError(io_err.to_string()),
            _ => PdfError::InvalidPdf(err.to_string()),
        }
    }
}

/// Convenience `Result` alias for PDF extraction operations.
pub type Result<T> = std::result::Result<T, PdfError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_pdf_error() {
        let err = PdfError::InvalidPdf("corrupted header".to_string());
        assert_eq!(err.to_string(), "Invalid PDF: corrupted header");
    }

    #[test]
    fn test_password_required_error() {
        let err = PdfError::PasswordRequired;
        assert_eq!(err.to_string(), "PDF is password-protected");
    }

    #[test]
    fn test_invalid_password_error() {
        let err = PdfError::InvalidPassword;
        assert_eq!(err.to_string(), "Invalid password provided");
    }

    #[test]
    fn test_encryption_not_supported_error() {
        let err = PdfError::EncryptionNotSupported("AES-256".to_string());
        assert_eq!(err.to_string(), "Encryption not supported: AES-256");
    }

    #[test]
    fn test_page_not_found_error() {
        let err = PdfError::PageNotFound(5);
        assert_eq!(err.to_string(), "Page 5 not found");
    }

    #[test]
    fn test_text_extraction_failed_error() {
        let err = PdfError::TextExtractionFailed("no text layer".to_string());
        assert_eq!(err.to_string(), "Text extraction failed: no text layer");
    }

    #[test]
    fn test_rendering_failed_error() {
        let err = PdfError::RenderingFailed("out of memory".to_string());
        assert_eq!(err.to_string(), "Page rendering failed: out of memory");
    }

    #[test]
    fn test_metadata_extraction_failed_error() {
        let err = PdfError::MetadataExtractionFailed("invalid metadata".to_string());
        assert_eq!(err.to_string(), "Metadata extraction failed: invalid metadata");
    }

    #[test]
    fn test_io_error() {
        let err = PdfError::IOError("read failed".to_string());
        assert_eq!(err.to_string(), "I/O error: read failed");
    }

    #[test]
    fn test_error_debug() {
        let err = PdfError::InvalidPassword;
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("InvalidPassword"));
    }

    #[test]
    fn test_error_clone() {
        let err1 = PdfError::PageNotFound(3);
        let err2 = err1.clone();
        assert_eq!(err1.to_string(), err2.to_string());
    }

    #[test]
    fn test_extraction_failed_error() {
        let err = PdfError::ExtractionFailed("page data mismatch".to_string());
        assert_eq!(err.to_string(), "Extraction failed: page data mismatch");
    }

    #[test]
    fn test_font_loading_failed_error() {
        let err = PdfError::FontLoadingFailed("missing font file".to_string());
        assert_eq!(err.to_string(), "Font loading failed: missing font file");
    }
}
