//! Excel spreadsheet extractor.

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::ExtractionResult;
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::Path;

/// Excel spreadsheet extractor using calamine.
///
/// Supports: .xlsx, .xlsm, .xlam, .xltm, .xls, .xla, .xlsb, .ods
pub struct ExcelExtractor;

impl Default for ExcelExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl ExcelExtractor {
    pub fn new() -> Self {
        Self
    }
}

impl Plugin for ExcelExtractor {
    fn name(&self) -> &str {
        "excel-extractor"
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
impl DocumentExtractor for ExcelExtractor {
    async fn extract_bytes(
        &self,
        content: &[u8],
        mime_type: &str,
        _config: &ExtractionConfig,
    ) -> Result<ExtractionResult> {
        // Determine file extension from MIME type
        let extension = match mime_type {
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" => ".xlsx",
            "application/vnd.ms-excel.sheet.macroEnabled.12" => ".xlsm",
            "application/vnd.ms-excel.addin.macroEnabled.12" => ".xlam",
            "application/vnd.ms-excel.template.macroEnabled.12" => ".xltm",
            "application/vnd.ms-excel" => ".xls",
            "application/vnd.ms-excel.addin.macroEnabled" => ".xla",
            "application/vnd.ms-excel.sheet.binary.macroEnabled.12" => ".xlsb",
            "application/vnd.oasis.opendocument.spreadsheet" => ".ods",
            _ => ".xlsx", // Default fallback
        };

        // Read Excel workbook
        let workbook = crate::extraction::excel::read_excel_bytes(content, extension)?;

        // Convert to markdown
        let markdown = crate::extraction::excel::excel_to_markdown(&workbook);

        // Build metadata
        let mut metadata = HashMap::new();
        for (key, value) in &workbook.metadata {
            metadata.insert(key.clone(), serde_json::json!(value));
        }

        Ok(ExtractionResult {
            content: markdown,
            mime_type: mime_type.to_string(),
            metadata,
            tables: vec![],
        })
    }

    async fn extract_file(&self, path: &Path, mime_type: &str, _config: &ExtractionConfig) -> Result<ExtractionResult> {
        // Use file-based extraction for better performance
        let path_str = path
            .to_str()
            .ok_or_else(|| crate::KreuzbergError::validation("Invalid file path".to_string()))?;

        let workbook = crate::extraction::excel::read_excel_file(path_str)?;
        let markdown = crate::extraction::excel::excel_to_markdown(&workbook);

        let mut metadata = HashMap::new();
        for (key, value) in &workbook.metadata {
            metadata.insert(key.clone(), serde_json::json!(value));
        }

        Ok(ExtractionResult {
            content: markdown,
            mime_type: mime_type.to_string(),
            metadata,
            tables: vec![],
        })
    }

    fn supported_mime_types(&self) -> &[&str] {
        &[
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            "application/vnd.ms-excel.sheet.macroEnabled.12",
            "application/vnd.ms-excel.addin.macroEnabled.12",
            "application/vnd.ms-excel.template.macroEnabled.12",
            "application/vnd.ms-excel",
            "application/vnd.ms-excel.addin.macroEnabled",
            "application/vnd.ms-excel.sheet.binary.macroEnabled.12",
            "application/vnd.oasis.opendocument.spreadsheet",
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
    fn test_excel_extractor_plugin_interface() {
        let extractor = ExcelExtractor::new();
        assert_eq!(extractor.name(), "excel-extractor");
        assert!(extractor.initialize().is_ok());
        assert!(extractor.shutdown().is_ok());
    }

    #[test]
    fn test_excel_extractor_supported_mime_types() {
        let extractor = ExcelExtractor::new();
        let mime_types = extractor.supported_mime_types();
        assert_eq!(mime_types.len(), 8);
        assert!(mime_types.contains(&"application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"));
        assert!(mime_types.contains(&"application/vnd.ms-excel"));
    }
}
