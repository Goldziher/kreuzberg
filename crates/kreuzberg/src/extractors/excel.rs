//! Excel spreadsheet extractor.

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::{ExcelMetadata, ExtractionResult, Metadata, Table};
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

    /// Convert Excel workbook sheets to Table structs.
    ///
    /// Each sheet becomes a table with the first row as headers,
    /// remaining rows as data, and the sheet name as caption.
    fn sheets_to_tables(workbook: &crate::types::ExcelWorkbook) -> Vec<Table> {
        let mut tables = Vec::with_capacity(workbook.sheets.len());

        for (sheet_index, sheet) in workbook.sheets.iter().enumerate() {
            if sheet.row_count == 0 || sheet.col_count == 0 {
                continue;
            }

            let lines: Vec<&str> = sheet.markdown.lines().collect();
            let mut cells: Vec<Vec<String>> = Vec::new();

            let table_start = lines.iter().position(|line| line.starts_with("| "));

            if let Some(start_idx) = table_start {
                for line in lines.iter().skip(start_idx) {
                    if line.starts_with("| ") && !line.contains("---") {
                        let row: Vec<String> = line
                            .trim_start_matches("| ")
                            .trim_end_matches(" |")
                            .split(" | ")
                            .map(|cell| cell.replace("\\|", "|").replace("\\\\", "\\"))
                            .collect();
                        cells.push(row);
                    }
                }
            }

            if !cells.is_empty() {
                tables.push(Table {
                    cells,
                    markdown: sheet.markdown.clone(),
                    page_number: sheet_index + 1,
                });
            }
        }

        tables
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
    #[cfg_attr(feature = "otel", tracing::instrument(
        skip(self, content, _config),
        fields(
            extractor.name = self.name(),
            content.size_bytes = content.len(),
        )
    ))]
    async fn extract_bytes(
        &self,
        content: &[u8],
        mime_type: &str,
        _config: &ExtractionConfig,
    ) -> Result<ExtractionResult> {
        let extension = match mime_type {
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" => ".xlsx",
            "application/vnd.ms-excel.sheet.macroEnabled.12" => ".xlsm",
            "application/vnd.ms-excel.addin.macroEnabled.12" => ".xlam",
            "application/vnd.ms-excel.template.macroEnabled.12" => ".xltm",
            "application/vnd.ms-excel" => ".xls",
            "application/vnd.ms-excel.addin.macroEnabled" => ".xla",
            "application/vnd.ms-excel.sheet.binary.macroEnabled.12" => ".xlsb",
            "application/vnd.oasis.opendocument.spreadsheet" => ".ods",
            _ => ".xlsx",
        };

        let workbook = if crate::core::batch_mode::is_batch_mode() {
            let content_owned = content.to_vec();
            let extension_owned = extension.to_string();
            let span = tracing::Span::current();
            tokio::task::spawn_blocking(move || {
                let _guard = span.entered();
                crate::extraction::excel::read_excel_bytes(&content_owned, &extension_owned)
            })
            .await
            .map_err(|e| crate::error::KreuzbergError::parsing(format!("Excel extraction task failed: {}", e)))??
        } else {
            crate::extraction::excel::read_excel_bytes(content, extension)?
        };

        let markdown = crate::extraction::excel::excel_to_markdown(&workbook);
        let tables = Self::sheets_to_tables(&workbook);

        let sheet_names: Vec<String> = workbook.sheets.iter().map(|s| s.name.clone()).collect();
        let excel_metadata = ExcelMetadata {
            sheet_count: workbook.sheets.len(),
            sheet_names,
        };

        let mut additional = std::collections::HashMap::new();
        for (key, value) in &workbook.metadata {
            if key != "sheet_count" && key != "sheet_names" {
                additional.insert(key.clone(), serde_json::json!(value));
            }
        }

        Ok(ExtractionResult {
            content: markdown,
            mime_type: mime_type.to_string(),
            metadata: Metadata {
                format: Some(crate::types::FormatMetadata::Excel(excel_metadata)),
                additional,
                ..Default::default()
            },
            tables,
            detected_languages: None,
            chunks: None,
            images: None,
        })
    }

    #[cfg_attr(feature = "otel", tracing::instrument(
        skip(self, path, _config),
        fields(
            extractor.name = self.name(),
        )
    ))]
    async fn extract_file(&self, path: &Path, mime_type: &str, _config: &ExtractionConfig) -> Result<ExtractionResult> {
        let path_str = path
            .to_str()
            .ok_or_else(|| crate::KreuzbergError::validation("Invalid file path".to_string()))?;

        let workbook = crate::extraction::excel::read_excel_file(path_str)?;
        let markdown = crate::extraction::excel::excel_to_markdown(&workbook);
        let tables = Self::sheets_to_tables(&workbook);

        let sheet_names: Vec<String> = workbook.sheets.iter().map(|s| s.name.clone()).collect();
        let excel_metadata = ExcelMetadata {
            sheet_count: workbook.sheets.len(),
            sheet_names,
        };

        let mut additional = std::collections::HashMap::new();
        for (key, value) in &workbook.metadata {
            if key != "sheet_count" && key != "sheet_names" {
                additional.insert(key.clone(), serde_json::json!(value));
            }
        }

        Ok(ExtractionResult {
            content: markdown,
            mime_type: mime_type.to_string(),
            metadata: Metadata {
                format: Some(crate::types::FormatMetadata::Excel(excel_metadata)),
                additional,
                ..Default::default()
            },
            tables,
            detected_languages: None,
            chunks: None,
            images: None,
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

    #[test]
    fn test_sheets_to_tables_conversion() {
        use crate::types::ExcelSheet;
        use std::collections::HashMap;

        let sheet = ExcelSheet {
            name: "TestSheet".to_string(),
            markdown: r#"## TestSheet

| Name | Age | City |
| --- | --- | --- |
| Alice | 30 | NYC |
| Bob | 25 | LA |
"#
            .to_string(),
            row_count: 3,
            col_count: 3,
            cell_count: 9,
        };

        let workbook = crate::types::ExcelWorkbook {
            sheets: vec![sheet],
            metadata: HashMap::new(),
        };

        let tables = ExcelExtractor::sheets_to_tables(&workbook);

        assert_eq!(tables.len(), 1);
        assert_eq!(tables[0].page_number, 1);
        assert_eq!(tables[0].cells.len(), 3);
        assert_eq!(tables[0].cells[0], vec!["Name", "Age", "City"]);
        assert_eq!(tables[0].cells[1], vec!["Alice", "30", "NYC"]);
        assert_eq!(tables[0].cells[2], vec!["Bob", "25", "LA"]);
    }

    #[test]
    fn test_sheets_to_tables_empty_sheet() {
        use crate::types::ExcelSheet;
        use std::collections::HashMap;

        let sheet = ExcelSheet {
            name: "EmptySheet".to_string(),
            markdown: "## EmptySheet\n\n*Empty sheet*".to_string(),
            row_count: 0,
            col_count: 0,
            cell_count: 0,
        };

        let workbook = crate::types::ExcelWorkbook {
            sheets: vec![sheet],
            metadata: HashMap::new(),
        };

        let tables = ExcelExtractor::sheets_to_tables(&workbook);
        assert_eq!(tables.len(), 0);
    }

    #[test]
    fn test_sheets_to_tables_multiple_sheets() {
        use crate::types::ExcelSheet;
        use std::collections::HashMap;

        let sheet1 = ExcelSheet {
            name: "Sheet1".to_string(),
            markdown: r#"## Sheet1

| Col1 | Col2 |
| --- | --- |
| A | B |
"#
            .to_string(),
            row_count: 2,
            col_count: 2,
            cell_count: 4,
        };

        let sheet2 = ExcelSheet {
            name: "Sheet2".to_string(),
            markdown: r#"## Sheet2

| X | Y |
| --- | --- |
| 1 | 2 |
"#
            .to_string(),
            row_count: 2,
            col_count: 2,
            cell_count: 4,
        };

        let workbook = crate::types::ExcelWorkbook {
            sheets: vec![sheet1, sheet2],
            metadata: HashMap::new(),
        };

        let tables = ExcelExtractor::sheets_to_tables(&workbook);

        assert_eq!(tables.len(), 2);
        assert_eq!(tables[0].page_number, 1);
        assert_eq!(tables[1].page_number, 2);
    }
}
