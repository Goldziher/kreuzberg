//! hOCR to Markdown conversion
//!
//! This module provides functionality to convert hOCR (HTML-formatted OCR output)
//! to clean Markdown text.

use html_to_markdown::{ConversionOptions, convert};

use super::error::OCRError;

/// Convert hOCR HTML to Markdown
///
/// Takes hOCR output from Tesseract and converts it to clean Markdown format.
///
/// # Arguments
///
/// * `hocr_html` - The hOCR HTML string from Tesseract
/// * `options` - Optional conversion options for customizing output
///
/// # Returns
///
/// Markdown-formatted text extracted from hOCR
///
/// # Example
///
/// ```no_run
/// use kreuzberg::ocr::hocr::convert_hocr_to_markdown;
///
/// let hocr = r#"<div class="ocr_page">
///     <div class="ocr_carea">
///         <p class="ocr_par">
///             <span class="ocrx_word">Hello</span>
///             <span class="ocrx_word">World</span>
///         </p>
///     </div>
/// </div>"#;
///
/// let markdown = convert_hocr_to_markdown(hocr, None).unwrap();
/// assert!(markdown.contains("Hello World"));
/// ```
pub fn convert_hocr_to_markdown(hocr_html: &str, options: Option<ConversionOptions>) -> Result<String, OCRError> {
    convert(hocr_html, options).map_err(|e| OCRError::ProcessingFailed(format!("hOCR conversion failed: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_hocr_conversion() {
        let hocr = r#"<div class="ocr_page">
            <p class="ocr_par">
                <span class="ocrx_word">Hello</span>
                <span class="ocrx_word">World</span>
            </p>
        </div>"#;

        let markdown = convert_hocr_to_markdown(hocr, None).unwrap();
        assert!(markdown.contains("Hello"));
        assert!(markdown.contains("World"));
    }

    #[test]
    fn test_hocr_with_formatting() {
        let hocr = r#"<div class="ocr_page">
            <p class="ocr_par">
                <strong class="ocrx_word">Bold</strong>
                <em class="ocrx_word">Italic</em>
            </p>
        </div>"#;

        let markdown = convert_hocr_to_markdown(hocr, None).unwrap();
        // Should preserve formatting
        assert!(markdown.len() > 0);
    }

    #[test]
    fn test_empty_hocr() {
        let hocr = "";
        let markdown = convert_hocr_to_markdown(hocr, None).unwrap();
        assert!(markdown.is_empty() || markdown.trim().is_empty());
    }

    #[test]
    fn test_hocr_with_headings() {
        let hocr = r#"<div class="ocr_page">
            <h1>Title</h1>
            <p class="ocr_par">
                <span class="ocrx_word">Content</span>
            </p>
        </div>"#;

        let markdown = convert_hocr_to_markdown(hocr, None).unwrap();
        assert!(markdown.contains("Title"));
        assert!(markdown.contains("Content"));
    }
}
