use super::error::{PdfError, Result};
use crate::core::config::PageConfig;
use crate::types::{PageBoundary, PageContent};
use pdfium_render::prelude::*;

/// Result type for PDF text extraction with optional page tracking.
#[allow(dead_code)]
type PdfTextExtractionResult = (String, Option<Vec<PageBoundary>>, Option<Vec<PageContent>>);

pub struct PdfTextExtractor {
    pdfium: Pdfium,
}

impl PdfTextExtractor {
    pub fn new() -> Result<Self> {
        let binding = Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./"))
            .or_else(|_| Pdfium::bind_to_system_library())
            .map_err(|e| PdfError::TextExtractionFailed(format!("Failed to initialize Pdfium: {}", e)))?;

        let pdfium = Pdfium::new(binding);
        Ok(Self { pdfium })
    }

    pub fn extract_text(&self, pdf_bytes: &[u8]) -> Result<String> {
        self.extract_text_with_password(pdf_bytes, None)
    }

    pub fn extract_text_with_password(&self, pdf_bytes: &[u8], password: Option<&str>) -> Result<String> {
        let document = self.pdfium.load_pdf_from_byte_slice(pdf_bytes, password).map_err(|e| {
            let err_msg = e.to_string();
            if (err_msg.contains("password") || err_msg.contains("Password")) && password.is_some() {
                PdfError::InvalidPassword
            } else if err_msg.contains("password") || err_msg.contains("Password") {
                PdfError::PasswordRequired
            } else {
                PdfError::InvalidPdf(err_msg)
            }
        })?;

        let (content, _, _) = extract_text_from_pdf_document(&document, None)?;
        Ok(content)
    }

    pub fn extract_text_with_passwords(&self, pdf_bytes: &[u8], passwords: &[&str]) -> Result<String> {
        let mut last_error = None;

        for password in passwords {
            match self.extract_text_with_password(pdf_bytes, Some(password)) {
                Ok(text) => return Ok(text),
                Err(e) => {
                    last_error = Some(e);
                    continue;
                }
            }
        }

        if let Some(err) = last_error {
            return Err(err);
        }

        self.extract_text(pdf_bytes)
    }

    pub fn get_page_count(&self, pdf_bytes: &[u8]) -> Result<usize> {
        let document = self.pdfium.load_pdf_from_byte_slice(pdf_bytes, None).map_err(|e| {
            let err_msg = e.to_string();
            if err_msg.contains("password") || err_msg.contains("Password") {
                PdfError::PasswordRequired
            } else {
                PdfError::InvalidPdf(err_msg)
            }
        })?;

        Ok(document.pages().len() as usize)
    }
}

impl Default for PdfTextExtractor {
    fn default() -> Self {
        Self::new().expect("Failed to create PDF text extractor")
    }
}

pub fn extract_text_from_pdf(pdf_bytes: &[u8]) -> Result<String> {
    let extractor = PdfTextExtractor::new()?;
    extractor.extract_text(pdf_bytes)
}

pub fn extract_text_from_pdf_with_password(pdf_bytes: &[u8], password: &str) -> Result<String> {
    let extractor = PdfTextExtractor::new()?;
    extractor.extract_text_with_password(pdf_bytes, Some(password))
}

pub fn extract_text_from_pdf_with_passwords(pdf_bytes: &[u8], passwords: &[&str]) -> Result<String> {
    let extractor = PdfTextExtractor::new()?;
    extractor.extract_text_with_passwords(pdf_bytes, passwords)
}

/// Extract text from PDF document with optional page boundary tracking.
///
/// # Arguments
///
/// * `document` - The PDF document to extract text from
/// * `page_config` - Optional page configuration for boundary tracking and page markers
///
/// # Returns
///
/// A tuple containing:
/// - The extracted text content (String)
/// - Optional page boundaries when page tracking is enabled (Vec<PageBoundary>)
/// - Optional per-page content when extract_pages is enabled (Vec<PageContent>)
///
/// # Implementation Details
///
/// When page_config is None, returns fast path with (content, None, None).
/// When page_config is Some, tracks byte offsets using .len() for O(1) performance (UTF-8 valid boundaries).
pub fn extract_text_from_pdf_document(
    document: &PdfDocument<'_>,
    page_config: Option<&PageConfig>,
) -> Result<PdfTextExtractionResult> {
    let page_count = document.pages().len() as usize;

    // Fast path: no page tracking needed
    if page_config.is_none() {
        let estimated_size = page_count * 2048;
        let mut content = String::with_capacity(estimated_size);

        for page in document.pages().iter() {
            let text = page
                .text()
                .map_err(|e| PdfError::TextExtractionFailed(format!("Page text extraction failed: {}", e)))?;

            let page_text = text.all();

            if !content.is_empty() {
                content.push_str("\n\n");
            }
            content.push_str(&page_text);
        }

        content.shrink_to_fit();
        return Ok((content, None, None));
    }

    let config = page_config.unwrap();
    let estimated_size = page_count * 2048;
    let mut content = String::with_capacity(estimated_size);
    let mut boundaries = Vec::with_capacity(page_count);
    let mut page_contents = if config.extract_pages {
        Some(Vec::with_capacity(page_count))
    } else {
        None
    };

    for (page_idx, page) in document.pages().iter().enumerate() {
        let page_number = page_idx + 1;

        let text = page
            .text()
            .map_err(|e| PdfError::TextExtractionFailed(format!("Page text extraction failed: {}", e)))?;

        let page_text = text.all();

        // Insert page marker before pages 2+
        if page_number > 1 && config.insert_page_markers {
            let marker = config.marker_format.replace("{page_num}", &page_number.to_string());
            content.push_str(&marker);
        }

        // Add separator between pages 1 and 2+ (if no markers)
        if page_number > 1 && !config.insert_page_markers && !content.is_empty() {
            content.push_str("\n\n");
        }

        // Track byte offset before this page (using .len() for byte offsets, UTF-8 valid boundaries)
        let byte_start = content.len();

        // Add page text
        content.push_str(&page_text);

        // Track byte offset after this page
        let byte_end = content.len();

        // Record boundary
        boundaries.push(PageBoundary {
            byte_start,
            byte_end,
            page_number,
        });

        // Record page content if enabled
        if let Some(ref mut pages) = page_contents {
            pages.push(PageContent {
                page_number,
                content: page_text,
                tables: Vec::new(),
                images: Vec::new(),
            });
        }
    }

    content.shrink_to_fit();

    Ok((content, Some(boundaries), page_contents))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extractor_creation() {
        let result = PdfTextExtractor::new();
        assert!(result.is_ok());
    }

    #[test]
    fn test_extract_empty_pdf() {
        let extractor = PdfTextExtractor::new().unwrap();
        let result = extractor.extract_text(b"");
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_invalid_pdf() {
        let extractor = PdfTextExtractor::new().unwrap();
        let result = extractor.extract_text(b"not a pdf");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PdfError::InvalidPdf(_)));
    }

    #[test]
    fn test_password_required_detection() {
        let extractor = PdfTextExtractor::new().unwrap();
        let encrypted_pdf = b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n";
        let result = extractor.extract_text(encrypted_pdf);

        if let Err(err) = result {
            assert!(matches!(err, PdfError::PasswordRequired | PdfError::InvalidPdf(_)));
        }
    }

    #[test]
    fn test_extract_text_with_passwords_empty_list() {
        let extractor = PdfTextExtractor::new().unwrap();
        let result = extractor.extract_text_with_passwords(b"not a pdf", &[]);
        assert!(result.is_err());
    }
}
