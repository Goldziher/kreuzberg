//! PDF document extractor.

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::ExtractionResult;
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::Path;

/// PDF document extractor using pypdfium2 and playa-pdf.
pub struct PdfExtractor;

impl Default for PdfExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl PdfExtractor {
    pub fn new() -> Self {
        Self
    }
}

impl Plugin for PdfExtractor {
    fn name(&self) -> &str {
        "pdf-extractor"
    }

    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    fn initialize(&self) -> Result<()> {
        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

#[async_trait]
impl DocumentExtractor for PdfExtractor {
    async fn extract_bytes(
        &self,
        content: &[u8],
        mime_type: &str,
        _config: &ExtractionConfig,
    ) -> Result<ExtractionResult> {
        // Extract text using PDF module (password support requires pdf_config field - TODO in task #8)
        let text = crate::pdf::text::extract_text_from_pdf(content)?;

        // Extract metadata
        let pdf_metadata = crate::pdf::metadata::extract_metadata(content)?;

        // Convert metadata to HashMap
        let mut metadata = HashMap::new();
        if let Some(title) = pdf_metadata.title {
            metadata.insert("title".to_string(), serde_json::json!(title));
        }
        if let Some(authors) = pdf_metadata.authors
            && !authors.is_empty()
        {
            metadata.insert("authors".to_string(), serde_json::json!(authors));
        }
        if let Some(subject) = pdf_metadata.subject {
            metadata.insert("subject".to_string(), serde_json::json!(subject));
        }
        if let Some(keywords) = pdf_metadata.keywords
            && !keywords.is_empty()
        {
            metadata.insert("keywords".to_string(), serde_json::json!(keywords));
        }
        metadata.insert("page_count".to_string(), serde_json::json!(pdf_metadata.page_count));

        Ok(ExtractionResult {
            content: text,
            mime_type: mime_type.to_string(),
            metadata,
            tables: vec![],
        })
    }

    async fn extract_file(&self, path: &Path, mime_type: &str, config: &ExtractionConfig) -> Result<ExtractionResult> {
        let bytes = tokio::fs::read(path).await?;
        self.extract_bytes(&bytes, mime_type, config).await
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["application/pdf"]
    }

    fn priority(&self) -> i32 {
        50 // Default priority
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pdf_extractor_plugin_interface() {
        let extractor = PdfExtractor::new();
        assert_eq!(extractor.name(), "pdf-extractor");
        assert!(extractor.initialize().is_ok());
        assert!(extractor.shutdown().is_ok());
    }

    #[test]
    fn test_pdf_extractor_supported_mime_types() {
        let extractor = PdfExtractor::new();
        let mime_types = extractor.supported_mime_types();
        assert_eq!(mime_types.len(), 1);
        assert!(mime_types.contains(&"application/pdf"));
    }

    #[test]
    fn test_pdf_extractor_priority() {
        let extractor = PdfExtractor::new();
        assert_eq!(extractor.priority(), 50);
    }
}
