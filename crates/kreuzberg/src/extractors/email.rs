//! Email message extractor.

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::{EmailMetadata, ExtractionResult, Metadata};
use async_trait::async_trait;
use std::path::Path;

/// Email message extractor.
///
/// Supports: .eml, .msg
pub struct EmailExtractor;

impl Default for EmailExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl EmailExtractor {
    pub fn new() -> Self {
        Self
    }
}

impl Plugin for EmailExtractor {
    fn name(&self) -> &str {
        "email-extractor"
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
impl DocumentExtractor for EmailExtractor {
    async fn extract_bytes(
        &self,
        content: &[u8],
        mime_type: &str,
        _config: &ExtractionConfig,
    ) -> Result<ExtractionResult> {
        // Extract email content
        let email_result = crate::extraction::email::extract_email_content(content, mime_type)?;

        // Build text output
        let text = crate::extraction::email::build_email_text_output(&email_result);

        // Build typed metadata
        let attachment_names: Vec<String> = email_result
            .attachments
            .iter()
            .filter_map(|att| att.filename.clone().or_else(|| att.name.clone()))
            .collect();

        let email_metadata = EmailMetadata {
            from_email: email_result.from_email.clone(),
            from_name: None, // Not available in EmailExtractionResult
            to_emails: email_result.to_emails.clone(),
            cc_emails: email_result.cc_emails.clone(),
            bcc_emails: email_result.bcc_emails.clone(),
            message_id: email_result.message_id.clone(),
            attachments: attachment_names,
        };

        // Any additional metadata fields go into additional HashMap
        let mut additional = std::collections::HashMap::new();
        for (key, value) in &email_result.metadata {
            additional.insert(key.clone(), serde_json::json!(value));
        }

        Ok(ExtractionResult {
            content: text,
            mime_type: mime_type.to_string(),
            metadata: Metadata {
                email: Some(email_metadata),
                subject: email_result.subject.clone(),
                date: email_result.date.clone(),
                additional,
                ..Default::default()
            },
            tables: vec![],
            detected_languages: None,
            chunks: None,
        })
    }

    async fn extract_file(&self, path: &Path, mime_type: &str, config: &ExtractionConfig) -> Result<ExtractionResult> {
        let bytes = tokio::fs::read(path).await?;
        self.extract_bytes(&bytes, mime_type, config).await
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["message/rfc822", "application/vnd.ms-outlook"]
    }

    fn priority(&self) -> i32 {
        50
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_extractor_plugin_interface() {
        let extractor = EmailExtractor::new();
        assert_eq!(extractor.name(), "email-extractor");
        assert!(extractor.initialize().is_ok());
        assert!(extractor.shutdown().is_ok());
    }

    #[test]
    fn test_email_extractor_supported_mime_types() {
        let extractor = EmailExtractor::new();
        let mime_types = extractor.supported_mime_types();
        assert_eq!(mime_types.len(), 2);
        assert!(mime_types.contains(&"message/rfc822"));
        assert!(mime_types.contains(&"application/vnd.ms-outlook"));
    }
}
