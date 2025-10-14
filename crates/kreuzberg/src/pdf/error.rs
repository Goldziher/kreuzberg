use std::fmt;

#[derive(Debug, Clone)]
pub enum PdfError {
    InvalidPdf(String),
    PasswordRequired,
    InvalidPassword,
    EncryptionNotSupported(String),
    PageNotFound(usize),
    TextExtractionFailed(String),
    RenderingFailed(String),
    MetadataExtractionFailed(String),
    IOError(String),
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
            PdfError::IOError(msg) => write!(f, "I/O error: {}", msg),
        }
    }
}

impl std::error::Error for PdfError {}

impl From<std::io::Error> for PdfError {
    fn from(err: std::io::Error) -> Self {
        PdfError::IOError(err.to_string())
    }
}

impl From<lopdf::Error> for PdfError {
    fn from(err: lopdf::Error) -> Self {
        match err {
            lopdf::Error::IO(e) => PdfError::IOError(e.to_string()),
            _ => PdfError::InvalidPdf(err.to_string()),
        }
    }
}

pub type Result<T> = std::result::Result<T, PdfError>;
