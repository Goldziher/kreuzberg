use super::error::{PdfError, Result};
use lopdf::Document;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfImage {
    pub page_number: usize,
    pub image_index: usize,
    pub width: i64,
    pub height: i64,
    pub color_space: Option<String>,
    pub bits_per_component: Option<i64>,
    pub filters: Vec<String>,
    pub data: Vec<u8>,
}

#[derive(Debug)]
pub struct PdfImageExtractor {
    document: Document,
}

impl PdfImageExtractor {
    pub fn new(pdf_bytes: &[u8]) -> Result<Self> {
        Self::new_with_password(pdf_bytes, None)
    }

    pub fn new_with_password(pdf_bytes: &[u8], password: Option<&str>) -> Result<Self> {
        let mut doc =
            Document::load_mem(pdf_bytes).map_err(|e| PdfError::InvalidPdf(format!("Failed to load PDF: {}", e)))?;

        if doc.is_encrypted() {
            if let Some(pwd) = password {
                doc.decrypt(pwd).map_err(|_| PdfError::InvalidPassword)?;
            } else {
                return Err(PdfError::PasswordRequired);
            }
        }

        Ok(Self { document: doc })
    }

    pub fn extract_images(&self) -> Result<Vec<PdfImage>> {
        let mut all_images = Vec::new();
        let pages = self.document.get_pages();

        for (page_num, page_id) in pages.iter() {
            let images = self
                .document
                .get_page_images(*page_id)
                .map_err(|e| PdfError::MetadataExtractionFailed(format!("Failed to get page images: {}", e)))?;

            for (img_index, img) in images.iter().enumerate() {
                let filters = img.filters.clone().unwrap_or_default();

                all_images.push(PdfImage {
                    page_number: *page_num as usize,
                    image_index: img_index + 1,
                    width: img.width,
                    height: img.height,
                    color_space: img.color_space.clone(),
                    bits_per_component: img.bits_per_component,
                    filters,
                    data: img.content.to_vec(),
                });
            }
        }

        Ok(all_images)
    }

    pub fn extract_images_from_page(&self, page_number: u32) -> Result<Vec<PdfImage>> {
        let pages = self.document.get_pages();
        let page_id = pages
            .get(&page_number)
            .ok_or_else(|| PdfError::PageNotFound(page_number as usize))?;

        let images = self
            .document
            .get_page_images(*page_id)
            .map_err(|e| PdfError::MetadataExtractionFailed(format!("Failed to get page images: {}", e)))?;

        let mut page_images = Vec::new();
        for (img_index, img) in images.iter().enumerate() {
            let filters = img.filters.clone().unwrap_or_default();

            page_images.push(PdfImage {
                page_number: page_number as usize,
                image_index: img_index + 1,
                width: img.width,
                height: img.height,
                color_space: img.color_space.clone(),
                bits_per_component: img.bits_per_component,
                filters,
                data: img.content.to_vec(),
            });
        }

        Ok(page_images)
    }

    pub fn get_image_count(&self) -> Result<usize> {
        let images = self.extract_images()?;
        Ok(images.len())
    }
}

pub fn extract_images_from_pdf(pdf_bytes: &[u8]) -> Result<Vec<PdfImage>> {
    let extractor = PdfImageExtractor::new(pdf_bytes)?;
    extractor.extract_images()
}

pub fn extract_images_from_pdf_with_password(pdf_bytes: &[u8], password: &str) -> Result<Vec<PdfImage>> {
    let extractor = PdfImageExtractor::new_with_password(pdf_bytes, Some(password))?;
    extractor.extract_images()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extractor_creation() {
        let result = PdfImageExtractor::new(b"not a pdf");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PdfError::InvalidPdf(_)));
    }

    #[test]
    fn test_extract_images_invalid_pdf() {
        let result = extract_images_from_pdf(b"not a pdf");
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_images_empty_pdf() {
        let result = extract_images_from_pdf(b"");
        assert!(result.is_err());
    }
}
