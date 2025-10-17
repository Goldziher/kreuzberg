//! Excel spreadsheet extractor.

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::{ExcelMetadata, ExtractionResult, Metadata};
use async_trait::async_trait;
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

        // Build typed metadata
        let sheet_names: Vec<String> = workbook.sheets.iter().map(|s| s.name.clone()).collect();
        let excel_metadata = ExcelMetadata {
            sheet_count: workbook.sheets.len(),
            sheet_names,
        };

        // Any additional metadata fields go into additional HashMap
        let mut additional = std::collections::HashMap::new();
        for (key, value) in &workbook.metadata {
            // Skip the fields we've already extracted into typed metadata
            if key != "sheet_count" && key != "sheet_names" {
                additional.insert(key.clone(), serde_json::json!(value));
            }
        }

        Ok(ExtractionResult {
            content: markdown,
            mime_type: mime_type.to_string(),
            metadata: Metadata {
                excel: Some(excel_metadata),
                additional,
                ..Default::default()
            },
            tables: vec![],
            detected_languages: None,
        })
    }

    async fn extract_file(&self, path: &Path, mime_type: &str, _config: &ExtractionConfig) -> Result<ExtractionResult> {
        // Use file-based extraction for better performance
        let path_str = path
            .to_str()
            .ok_or_else(|| crate::KreuzbergError::validation("Invalid file path".to_string()))?;

        let workbook = crate::extraction::excel::read_excel_file(path_str)?;
        let markdown = crate::extraction::excel::excel_to_markdown(&workbook);

        // Build typed metadata
        let sheet_names: Vec<String> = workbook.sheets.iter().map(|s| s.name.clone()).collect();
        let excel_metadata = ExcelMetadata {
            sheet_count: workbook.sheets.len(),
            sheet_names,
        };

        // Any additional metadata fields go into additional HashMap
        let mut additional = std::collections::HashMap::new();
        for (key, value) in &workbook.metadata {
            // Skip the fields we've already extracted into typed metadata
            if key != "sheet_count" && key != "sheet_names" {
                additional.insert(key.clone(), serde_json::json!(value));
            }
        }

        Ok(ExtractionResult {
            content: markdown,
            mime_type: mime_type.to_string(),
            metadata: Metadata {
                excel: Some(excel_metadata),
                additional,
                ..Default::default()
            },
            tables: vec![],
            detected_languages: None,
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
