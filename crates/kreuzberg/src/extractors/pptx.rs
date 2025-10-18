//! PowerPoint presentation extractor.

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::{ExtractionResult, Metadata};
use async_trait::async_trait;
use std::path::Path;

/// PowerPoint presentation extractor.
///
/// Supports: .pptx, .pptm, .ppsx
pub struct PptxExtractor;

impl Default for PptxExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl PptxExtractor {
    pub fn new() -> Self {
        Self
    }
}

impl Plugin for PptxExtractor {
    fn name(&self) -> &str {
        "pptx-extractor"
    }

    fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    fn initialize(&self) -> Result<()> {
        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

#[async_trait]
impl DocumentExtractor for PptxExtractor {
    async fn extract_bytes(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<ExtractionResult> {
        // Extract images based on config
        let extract_images = config.images.as_ref().is_some_and(|img| img.extract_images);

        let pptx_result = crate::extraction::pptx::extract_pptx_from_bytes(content, extract_images)?;

        // Use PptxMetadata directly, put counts in additional metadata
        let mut additional = std::collections::HashMap::new();
        additional.insert("slide_count".to_string(), serde_json::json!(pptx_result.slide_count));
        additional.insert("image_count".to_string(), serde_json::json!(pptx_result.image_count));
        additional.insert("table_count".to_string(), serde_json::json!(pptx_result.table_count));

        Ok(ExtractionResult {
            content: pptx_result.content,
            mime_type: mime_type.to_string(),
            metadata: Metadata {
                pptx: Some(pptx_result.metadata),
                additional,
                ..Default::default()
            },
            tables: vec![],
            detected_languages: None,
            chunks: None,
        })
    }

    async fn extract_file(&self, path: &Path, mime_type: &str, config: &ExtractionConfig) -> Result<ExtractionResult> {
        let path_str = path
            .to_str()
            .ok_or_else(|| crate::KreuzbergError::validation("Invalid file path".to_string()))?;

        let extract_images = config.images.as_ref().is_some_and(|img| img.extract_images);

        let pptx_result = crate::extraction::pptx::extract_pptx_from_path(path_str, extract_images)?;

        // Use PptxMetadata directly, put counts in additional metadata
        let mut additional = std::collections::HashMap::new();
        additional.insert("slide_count".to_string(), serde_json::json!(pptx_result.slide_count));
        additional.insert("image_count".to_string(), serde_json::json!(pptx_result.image_count));
        additional.insert("table_count".to_string(), serde_json::json!(pptx_result.table_count));

        Ok(ExtractionResult {
            content: pptx_result.content,
            mime_type: mime_type.to_string(),
            metadata: Metadata {
                pptx: Some(pptx_result.metadata),
                additional,
                ..Default::default()
            },
            tables: vec![],
            detected_languages: None,
            chunks: None,
        })
    }

    fn supported_mime_types(&self) -> &[&str] {
        &[
            "application/vnd.openxmlformats-officedocument.presentationml.presentation",
            "application/vnd.ms-powerpoint.presentation.macroEnabled.12",
            "application/vnd.openxmlformats-officedocument.presentationml.slideshow",
        ]
    }

    fn priority(&self) -> i32 {
        50
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pptx_extractor_plugin_interface() {
        let extractor = PptxExtractor::new();
        assert_eq!(extractor.name(), "pptx-extractor");
        assert!(extractor.initialize().is_ok());
        assert!(extractor.shutdown().is_ok());
    }

    #[test]
    fn test_pptx_extractor_supported_mime_types() {
        let extractor = PptxExtractor::new();
        let mime_types = extractor.supported_mime_types();
        assert_eq!(mime_types.len(), 3);
        assert!(mime_types.contains(&"application/vnd.openxmlformats-officedocument.presentationml.presentation"));
    }
}
