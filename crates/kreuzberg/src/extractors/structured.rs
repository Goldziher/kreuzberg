//! Structured data extractor (JSON, YAML, TOML).

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::{ExtractionResult, Metadata};
use async_trait::async_trait;
use std::path::Path;

/// Structured data extractor supporting JSON, YAML, and TOML.
pub struct StructuredExtractor;

impl Default for StructuredExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl StructuredExtractor {
    pub fn new() -> Self {
        Self
    }
}

impl Plugin for StructuredExtractor {
    fn name(&self) -> &str {
        "structured-extractor"
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
impl DocumentExtractor for StructuredExtractor {
    async fn extract_bytes(
        &self,
        content: &[u8],
        mime_type: &str,
        _config: &ExtractionConfig,
    ) -> Result<ExtractionResult> {
        let structured_result = match mime_type {
            "application/json" | "text/json" => crate::extraction::structured::parse_json(content, None)?,
            "application/x-yaml" | "text/yaml" | "text/x-yaml" => crate::extraction::structured::parse_yaml(content)?,
            "application/toml" | "text/toml" => crate::extraction::structured::parse_toml(content)?,
            _ => return Err(crate::KreuzbergError::UnsupportedFormat(mime_type.to_string())),
        };

        let mut additional = std::collections::HashMap::new();
        additional.insert(
            "field_count".to_string(),
            serde_json::json!(structured_result.text_fields.len()),
        );
        additional.insert("data_format".to_string(), serde_json::json!(structured_result.format));

        for (key, value) in structured_result.metadata {
            additional.insert(key, serde_json::json!(value));
        }

        Ok(ExtractionResult {
            content: structured_result.content,
            mime_type: mime_type.to_string(),
            metadata: Metadata {
                additional,
                ..Default::default()
            },
            tables: vec![],
            detected_languages: None,
            chunks: None,
            images: None,
        })
    }

    async fn extract_file(&self, path: &Path, mime_type: &str, config: &ExtractionConfig) -> Result<ExtractionResult> {
        let bytes = tokio::fs::read(path).await?;
        self.extract_bytes(&bytes, mime_type, config).await
    }

    fn supported_mime_types(&self) -> &[&str] {
        &[
            "application/json",
            "text/json",
            "application/x-yaml",
            "text/yaml",
            "text/x-yaml",
            "application/toml",
            "text/toml",
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
    fn test_structured_extractor_plugin_interface() {
        let extractor = StructuredExtractor::new();
        assert_eq!(extractor.name(), "structured-extractor");
        assert!(extractor.initialize().is_ok());
        assert!(extractor.shutdown().is_ok());
    }

    #[test]
    fn test_structured_extractor_supported_mime_types() {
        let extractor = StructuredExtractor::new();
        let mime_types = extractor.supported_mime_types();
        assert_eq!(mime_types.len(), 7);
        assert!(mime_types.contains(&"application/json"));
        assert!(mime_types.contains(&"application/x-yaml"));
        assert!(mime_types.contains(&"application/toml"));
    }
}
