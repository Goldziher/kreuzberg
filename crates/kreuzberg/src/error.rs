use thiserror::Error;

pub type Result<T> = std::result::Result<T, KreuzbergError>;

#[derive(Debug, Error)]
pub enum KreuzbergError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parsing error: {0}")]
    Parsing(String),

    #[error("OCR error: {0}")]
    Ocr(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Cache error: {0}")]
    Cache(String),

    #[error("Image processing error: {0}")]
    ImageProcessing(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

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
        KreuzbergError::Parsing(err.to_string())
    }
}

impl From<serde_json::Error> for KreuzbergError {
    fn from(err: serde_json::Error) -> Self {
        KreuzbergError::Serialization(err.to_string())
    }
}

impl From<rmp_serde::encode::Error> for KreuzbergError {
    fn from(err: rmp_serde::encode::Error) -> Self {
        KreuzbergError::Serialization(err.to_string())
    }
}

impl From<rmp_serde::decode::Error> for KreuzbergError {
    fn from(err: rmp_serde::decode::Error) -> Self {
        KreuzbergError::Serialization(err.to_string())
    }
}
