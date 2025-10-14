use super::error::{PdfError, Result};
use pdfium_render::prelude::*;

pub struct PdfTextExtractor {
    pdfium: Pdfium,
}

impl PdfTextExtractor {
    pub fn new() -> Result<Self> {
        // Try to bind to the downloaded library first, fall back to system library
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

        let mut pages_content = Vec::new();

        for page in document.pages().iter() {
            let text = page
                .text()
                .map_err(|e| PdfError::TextExtractionFailed(format!("Page text extraction failed: {}", e)))?;

            pages_content.push(text.all());
        }

        Ok(pages_content.join("\n"))
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
