//! XML extractor.

use crate::core::config::ExtractionConfig;
use crate::extraction::xml::parse_xml;
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::ExtractionResult;
use crate::Result;
use async_trait::async_trait;

/// XML extractor.
///
/// Extracts text content from XML files, preserving element structure information.
pub struct XmlExtractor;

impl XmlExtractor {
    /// Create a new XML extractor.
    pub fn new() -> Self {
        Self
    }
}

impl Default for XmlExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for XmlExtractor {
    fn name(&self) -> &str {
        "xml-extractor"
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    fn initialize(&mut self) -> Result<()> {
        Ok(())
    }

    fn shutdown(&mut self) -> Result<()> {
        Ok(())
    }

    fn description(&self) -> &str {
        "Extracts text content from XML files with element metadata"
    }

    fn author(&self) -> &str {
        "Kreuzberg Team"
    }
}

#[async_trait]
impl DocumentExtractor for XmlExtractor {
    async fn extract_bytes(
        &self,
        content: &[u8],
        _mime_type: &str,
        _config: &ExtractionConfig,
    ) -> Result<ExtractionResult> {
        let xml_result = parse_xml(content, false)?;

        Ok(ExtractionResult {
            content: xml_result.content,
            mime_type: "application/xml".to_string(),
            metadata: std::collections::HashMap::from([
                ("element_count".to_string(), serde_json::json!(xml_result.element_count)),
                ("unique_elements".to_string(), serde_json::json!(xml_result.unique_elements)),
            ]),
            tables: vec![],
        })
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["application/xml", "text/xml", "image/svg+xml"]
    }

    fn priority(&self) -> i32 {
        50
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_xml_extractor() {
        let extractor = XmlExtractor::new();
        let content = b"<root><item>Hello</item><item>World</item></root>";
        let config = ExtractionConfig::default();

        let result = extractor
            .extract_bytes(content, "application/xml", &config)
            .await
            .unwrap();

        assert_eq!(result.mime_type, "application/xml");
        assert_eq!(result.content, "Hello World");
        assert!(result.metadata.contains_key("element_count"));
        assert!(result.metadata.contains_key("unique_elements"));
    }

    #[test]
    fn test_xml_plugin_interface() {
        let extractor = XmlExtractor::new();
        assert_eq!(extractor.name(), "xml-extractor");
        assert_eq!(extractor.version(), "1.0.0");
        assert_eq!(
            extractor.supported_mime_types(),
            &["application/xml", "text/xml", "image/svg+xml"]
        );
        assert_eq!(extractor.priority(), 50);
    }
}
