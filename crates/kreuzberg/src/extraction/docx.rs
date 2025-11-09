//! DOCX (Microsoft Word) text extraction using docx-lite.
//!
//! This module provides high-performance text extraction from DOCX files using the docx-lite
//! library, which uses streaming XML parsing for efficiency.

use crate::error::{KreuzbergError, Result};

/// Extract text from DOCX bytes using docx-lite.
///
/// # Arguments
/// * `bytes` - The DOCX file contents as bytes
///
/// # Returns
/// * `Ok(String)` - The extracted text content
/// * `Err(KreuzbergError)` - If extraction fails
///
/// # Performance
/// docx-lite uses streaming XML parsing for minimal memory overhead and high throughput
/// (~160 MB/s average). It eliminates subprocess overhead compared to Pandoc (~400x faster).
pub fn extract_text(bytes: &[u8]) -> Result<String> {
    docx_lite::extract_text_from_bytes(bytes)
        .map_err(|e| KreuzbergError::parsing(format!("DOCX text extraction failed: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_text_empty() {
        let result = extract_text(b"");
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_text_invalid() {
        let result = extract_text(b"not a docx file");
        assert!(result.is_err());
    }
}
