//! Docling DocTags extractor.
//!
//! Parses Docling DocTags documents through the extractor registry.

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::extractors::security::SecurityBudget;
use crate::plugins::{InternalDocumentExtractor, Plugin};
use crate::types::internal::InternalDocument;
use async_trait::async_trait;

/// Extractor for Docling DocTags documents.
///
/// Register this Rust-only extractor through the document-extractor registry.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Default)]
pub struct DocTagsExtractor;

impl DocTagsExtractor {
    /// Create a new DocTags extractor.
    pub fn new() -> Self {
        Self
    }
}

impl Plugin for DocTagsExtractor {
    fn name(&self) -> &str {
        "doctags-extractor"
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
        "Extracts content from Docling DocTags documents, including OTSL tables"
    }

    fn author(&self) -> &str {
        "Xberg Team"
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl InternalDocumentExtractor for DocTagsExtractor {
    async fn extract_content(
        &self,
        content: &[u8],
        _mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<InternalDocument> {
        tracing::debug!(format = "doctags", size_bytes = content.len(), "extraction starting");
        let mut budget = SecurityBudget::from_config(config);
        budget.account_text(content.len())?;

        let text = String::from_utf8_lossy(content);
        let text = crate::utils::strip_bom(&text);
        Ok(crate::extraction::doctags::parse_doctags(text))
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["text/vnd.docling.doctags", "application/vnd.docling.doctags"]
    }
}
