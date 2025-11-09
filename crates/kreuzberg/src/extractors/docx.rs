//! DOCX extractor using docx-lite for high-performance text extraction.
//!
//! Supports: Microsoft Word (.docx)

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::extraction::{docx, office_metadata};
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::{ExtractionResult, Metadata};
use async_trait::async_trait;
use std::io::Cursor;

/// High-performance DOCX extractor using docx-lite.
///
/// This extractor provides:
/// - Fast text extraction via streaming XML parsing (~160 MB/s average)
/// - Comprehensive metadata extraction (core.xml, app.xml, custom.xml)
/// - ~400x faster than Pandoc subprocess approach
pub struct DocxExtractor;

impl DocxExtractor {
    /// Create a new DOCX extractor.
    pub fn new() -> Self {
        Self
    }
}

impl Default for DocxExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for DocxExtractor {
    fn name(&self) -> &str {
        "docx-extractor"
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

    fn description(&self) -> &str {
        "High-performance DOCX text extraction using docx-lite with metadata support"
    }

    fn author(&self) -> &str {
        "Kreuzberg Team"
    }
}

#[async_trait]
impl DocumentExtractor for DocxExtractor {
    async fn extract_bytes(
        &self,
        content: &[u8],
        mime_type: &str,
        _config: &ExtractionConfig,
    ) -> Result<ExtractionResult> {
        // Extract text with docx-lite
        let text = if crate::core::batch_mode::is_batch_mode() {
            // Batch mode: Use spawn_blocking for parallelism
            let content_for_text = content.to_vec();
            tokio::task::spawn_blocking(move || docx::extract_text(&content_for_text))
                .await
                .map_err(|e| {
                    crate::error::KreuzbergError::parsing(format!("DOCX text extraction task failed: {}", e))
                })??
        } else {
            // Single-file mode: Direct extraction (no spawn overhead)
            docx::extract_text(content)?
        };

        // Extract metadata using existing office_metadata module
        let mut archive = if crate::core::batch_mode::is_batch_mode() {
            // Batch mode: Use spawn_blocking for parallelism
            let content_owned = content.to_vec();
            tokio::task::spawn_blocking(move || -> crate::error::Result<_> {
                let cursor = Cursor::new(content_owned);
                zip::ZipArchive::new(cursor)
                    .map_err(|e| crate::error::KreuzbergError::parsing(format!("Failed to open ZIP archive: {}", e)))
            })
            .await
            .map_err(|e| crate::error::KreuzbergError::parsing(format!("Task join error: {}", e)))??
        } else {
            // Single-file mode: Direct extraction (no spawn overhead)
            // Note: We still need to clone for ZipArchive type consistency with batch mode
            let content_owned = content.to_vec();
            let cursor = Cursor::new(content_owned);
            zip::ZipArchive::new(cursor)
                .map_err(|e| crate::error::KreuzbergError::parsing(format!("Failed to open ZIP archive: {}", e)))?
        };

        let mut metadata_map = std::collections::HashMap::new();

        // Extract core properties (title, creator, dates, keywords, etc.)
        if let Ok(core) = office_metadata::extract_core_properties(&mut archive) {
            if let Some(title) = core.title {
                metadata_map.insert("title".to_string(), serde_json::Value::String(title));
            }
            if let Some(creator) = core.creator {
                metadata_map.insert(
                    "authors".to_string(),
                    serde_json::Value::Array(vec![serde_json::Value::String(creator.clone())]),
                );
                metadata_map.insert("created_by".to_string(), serde_json::Value::String(creator));
            }
            if let Some(subject) = core.subject {
                metadata_map.insert("subject".to_string(), serde_json::Value::String(subject));
            }
            if let Some(keywords) = core.keywords {
                metadata_map.insert("keywords".to_string(), serde_json::Value::String(keywords));
            }
            if let Some(description) = core.description {
                metadata_map.insert("description".to_string(), serde_json::Value::String(description));
            }
            if let Some(modified_by) = core.last_modified_by {
                metadata_map.insert("modified_by".to_string(), serde_json::Value::String(modified_by));
            }
            if let Some(created) = core.created {
                metadata_map.insert("created_at".to_string(), serde_json::Value::String(created));
            }
            if let Some(modified) = core.modified {
                metadata_map.insert("modified_at".to_string(), serde_json::Value::String(modified));
            }
            if let Some(revision) = core.revision {
                metadata_map.insert("revision".to_string(), serde_json::Value::String(revision));
            }
            if let Some(category) = core.category {
                metadata_map.insert("category".to_string(), serde_json::Value::String(category));
            }
            if let Some(content_status) = core.content_status {
                metadata_map.insert("content_status".to_string(), serde_json::Value::String(content_status));
            }
            if let Some(language) = core.language {
                metadata_map.insert("language".to_string(), serde_json::Value::String(language));
            }
        }

        // Extract app properties (page count, word count, etc.)
        if let Ok(app) = office_metadata::extract_docx_app_properties(&mut archive) {
            if let Some(pages) = app.pages {
                metadata_map.insert("page_count".to_string(), serde_json::Value::Number(pages.into()));
            }
            if let Some(words) = app.words {
                metadata_map.insert("word_count".to_string(), serde_json::Value::Number(words.into()));
            }
            if let Some(chars) = app.characters {
                metadata_map.insert("character_count".to_string(), serde_json::Value::Number(chars.into()));
            }
            if let Some(lines) = app.lines {
                metadata_map.insert("line_count".to_string(), serde_json::Value::Number(lines.into()));
            }
            if let Some(paragraphs) = app.paragraphs {
                metadata_map.insert(
                    "paragraph_count".to_string(),
                    serde_json::Value::Number(paragraphs.into()),
                );
            }
            if let Some(template) = app.template {
                metadata_map.insert("template".to_string(), serde_json::Value::String(template));
            }
            if let Some(company) = app.company {
                metadata_map.insert("organization".to_string(), serde_json::Value::String(company));
            }
            if let Some(time) = app.total_time {
                metadata_map.insert(
                    "total_editing_time_minutes".to_string(),
                    serde_json::Value::Number(time.into()),
                );
            }
            if let Some(application) = app.application {
                metadata_map.insert("application".to_string(), serde_json::Value::String(application));
            }
        }

        // Extract custom properties
        if let Ok(custom) = office_metadata::extract_custom_properties(&mut archive) {
            for (key, value) in custom {
                metadata_map.insert(format!("custom_{}", key), value);
            }
        }

        Ok(ExtractionResult {
            content: text,
            mime_type: mime_type.to_string(),
            metadata: Metadata {
                additional: metadata_map,
                ..Default::default()
            },
            tables: vec![],
            detected_languages: None,
            chunks: None,
            images: None,
        })
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["application/vnd.openxmlformats-officedocument.wordprocessingml.document"]
    }

    fn priority(&self) -> i32 {
        50 // Higher priority than Pandoc (40) to take precedence
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_docx_extractor_plugin_interface() {
        let extractor = DocxExtractor::new();
        assert_eq!(extractor.name(), "docx-extractor");
        assert_eq!(extractor.version(), env!("CARGO_PKG_VERSION"));
        assert_eq!(extractor.priority(), 50);
        assert_eq!(extractor.supported_mime_types().len(), 1);
    }

    #[tokio::test]
    async fn test_docx_extractor_supports_docx() {
        let extractor = DocxExtractor::new();
        assert!(
            extractor
                .supported_mime_types()
                .contains(&"application/vnd.openxmlformats-officedocument.wordprocessingml.document")
        );
    }

    #[tokio::test]
    async fn test_docx_extractor_default() {
        let extractor = DocxExtractor;
        assert_eq!(extractor.name(), "docx-extractor");
    }

    #[tokio::test]
    async fn test_docx_extractor_initialize_shutdown() {
        let extractor = DocxExtractor::new();
        assert!(extractor.initialize().is_ok());
        assert!(extractor.shutdown().is_ok());
    }
}
