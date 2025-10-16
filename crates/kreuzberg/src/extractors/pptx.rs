//! PowerPoint presentation extractor.

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::ExtractionResult;
use async_trait::async_trait;
use std::collections::HashMap;
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
impl DocumentExtractor for PptxExtractor {
    async fn extract_bytes(
        &self,
        content: &[u8],
        mime_type: &str,
        _config: &ExtractionConfig,
    ) -> Result<ExtractionResult> {
        // Extract images based on config
        let extract_images = false; // TODO: Get from config when images field is added

        let pptx_result = crate::extraction::pptx::extract_pptx_from_bytes(content, extract_images)?;

        // Build metadata from PptxMetadata struct
        let mut metadata = HashMap::new();
        if let Some(title) = pptx_result.metadata.title {
            metadata.insert("title".to_string(), serde_json::json!(title));
        }
        if let Some(author) = pptx_result.metadata.author {
            metadata.insert("author".to_string(), serde_json::json!(author));
        }
        if let Some(description) = pptx_result.metadata.description {
            metadata.insert("description".to_string(), serde_json::json!(description));
        }
        if let Some(summary) = pptx_result.metadata.summary {
            metadata.insert("summary".to_string(), serde_json::json!(summary));
        }
        if !pptx_result.metadata.fonts.is_empty() {
            metadata.insert("fonts".to_string(), serde_json::json!(pptx_result.metadata.fonts));
        }
        metadata.insert("slide_count".to_string(), serde_json::json!(pptx_result.slide_count));
        metadata.insert("image_count".to_string(), serde_json::json!(pptx_result.image_count));
        metadata.insert("table_count".to_string(), serde_json::json!(pptx_result.table_count));

        Ok(ExtractionResult {
            content: pptx_result.content,
            mime_type: mime_type.to_string(),
            metadata,
            tables: vec![],
            detected_languages: None,
        })
    }

    async fn extract_file(&self, path: &Path, mime_type: &str, _config: &ExtractionConfig) -> Result<ExtractionResult> {
        let path_str = path
            .to_str()
            .ok_or_else(|| crate::KreuzbergError::validation("Invalid file path".to_string()))?;

        let extract_images = false; // TODO: Get from config

        let pptx_result = crate::extraction::pptx::extract_pptx_from_path(path_str, extract_images)?;

        // Build metadata from PptxMetadata struct
        let mut metadata = HashMap::new();
        if let Some(title) = pptx_result.metadata.title {
            metadata.insert("title".to_string(), serde_json::json!(title));
        }
        if let Some(author) = pptx_result.metadata.author {
            metadata.insert("author".to_string(), serde_json::json!(author));
        }
        if let Some(description) = pptx_result.metadata.description {
            metadata.insert("description".to_string(), serde_json::json!(description));
        }
        if let Some(summary) = pptx_result.metadata.summary {
            metadata.insert("summary".to_string(), serde_json::json!(summary));
        }
        if !pptx_result.metadata.fonts.is_empty() {
            metadata.insert("fonts".to_string(), serde_json::json!(pptx_result.metadata.fonts));
        }
        metadata.insert("slide_count".to_string(), serde_json::json!(pptx_result.slide_count));
        metadata.insert("image_count".to_string(), serde_json::json!(pptx_result.image_count));
        metadata.insert("table_count".to_string(), serde_json::json!(pptx_result.table_count));

        Ok(ExtractionResult {
            content: pptx_result.content,
            mime_type: mime_type.to_string(),
            metadata,
            tables: vec![],
            detected_languages: None,
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
