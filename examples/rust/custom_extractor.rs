//! Custom DocumentExtractor Example
//!
//! Demonstrates implementing a custom document extractor plugin in Rust.

use async_trait::async_trait;
use kreuzberg::{
    ExtractionConfig, ExtractionResult, KreuzbergError, Metadata, extract_file, extract_file_sync,
    plugins::{extractor::DocumentExtractor, registry::get_document_extractor_registry},
};
use std::sync::Arc;

/// Custom CSV extractor with advanced dialect detection.
///
/// This example shows how to implement a custom extractor for a file format
/// that isn't handled by the built-in extractors, or to replace a built-in
/// extractor with custom logic.
struct CSVExtractor {
    delimiter: Option<char>,
    has_headers: bool,
}

impl CSVExtractor {
    fn new() -> Self {
        Self {
            delimiter: None,
            has_headers: true,
        }
    }

    fn with_delimiter(mut self, delimiter: char) -> Self {
        self.delimiter = Some(delimiter);
        self
    }

    fn with_headers(mut self, has_headers: bool) -> Self {
        self.has_headers = has_headers;
        self
    }

    /// Detect CSV delimiter automatically
    fn detect_delimiter(&self, sample: &str) -> char {
        if let Some(delim) = self.delimiter {
            return delim;
        }

        let delimiters = [',', ';', '\t', '|'];
        let lines: Vec<&str> = sample.lines().take(5).collect();

        delimiters
            .iter()
            .max_by_key(|&&delim| {
                let counts: Vec<usize> = lines.iter().map(|line| line.matches(delim).count()).collect();
                if counts.is_empty() {
                    0
                } else {
                    let first = counts[0];
                    counts.iter().filter(|&&c| c == first && c > 0).count()
                }
            })
            .copied()
            .unwrap_or(',')
    }
}

#[async_trait]
impl DocumentExtractor for CSVExtractor {
    fn name(&self) -> &str {
        "csv_extractor"
    }

    fn supported_mime_types(&self) -> Vec<String> {
        vec![
            "text/csv".to_string(),
            "text/comma-separated-values".to_string(),
            "application/csv".to_string(),
        ]
    }

    fn priority(&self) -> i32 {
        100
    }

    async fn extract(
        &self,
        data: &[u8],
        _mime_type: &str,
        _config: &ExtractionConfig,
    ) -> Result<ExtractionResult, KreuzbergError> {
        let content = String::from_utf8(data.to_vec()).map_err(|e| KreuzbergError::Parsing {
            message: format!("Invalid UTF-8 in CSV file: {}", e),
            source: None,
        })?;

        let delimiter = self.detect_delimiter(&content);

        let mut lines = content.lines();
        let mut rows = Vec::new();
        let mut headers = Vec::new();

        if self.has_headers {
            if let Some(header_line) = lines.next() {
                headers = header_line.split(delimiter).map(|s| s.trim().to_string()).collect();
            }
        }

        for line in lines {
            let row: Vec<String> = line.split(delimiter).map(|s| s.trim().to_string()).collect();
            rows.push(row);
        }

        let mut markdown = String::new();

        if !headers.is_empty() {
            markdown.push_str("| ");
            markdown.push_str(&headers.join(" | "));
            markdown.push_str(" |\n");

            markdown.push_str("|");
            for _ in &headers {
                markdown.push_str(" --- |");
            }
            markdown.push_str("\n");
        }

        for row in &rows {
            markdown.push_str("| ");
            markdown.push_str(&row.join(" | "));
            markdown.push_str(" |\n");
        }

        let mut metadata = Metadata::default();
        metadata.format = Some("csv".to_string());

        let csv_metadata = serde_json::json!({
            "delimiter": delimiter.to_string(),
            "has_headers": self.has_headers,
            "row_count": rows.len(),
            "column_count": if !rows.is_empty() { rows[0].len() } else { 0 },
        });

        if let Ok(json_str) = serde_json::to_string(&csv_metadata) {
            metadata.format = Some(json_str);
        }

        Ok(ExtractionResult {
            content: markdown,
            mime_type: "text/csv".to_string(),
            metadata,
            tables: vec![],
            detected_languages: None,
            chunks: None,
        })
    }

    fn extract_sync(
        &self,
        data: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<ExtractionResult, KreuzbergError> {
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(self.extract(data, mime_type, config))
    }
}

/// Custom binary format extractor example.
struct BinaryFormatExtractor;

#[async_trait]
impl DocumentExtractor for BinaryFormatExtractor {
    fn name(&self) -> &str {
        "binary_format_extractor"
    }

    fn supported_mime_types(&self) -> Vec<String> {
        vec!["application/x-custom-binary".to_string()]
    }

    fn priority(&self) -> i32 {
        100
    }

    async fn extract(
        &self,
        data: &[u8],
        _mime_type: &str,
        _config: &ExtractionConfig,
    ) -> Result<ExtractionResult, KreuzbergError> {
        if data.len() < 4 {
            return Err(KreuzbergError::Parsing {
                message: "File too small".to_string(),
                source: None,
            });
        }

        if &data[0..4] != b"CUST" {
            return Err(KreuzbergError::Parsing {
                message: "Invalid magic bytes".to_string(),
                source: None,
            });
        }

        let content = format!("Binary file parsed: {} bytes", data.len());

        Ok(ExtractionResult {
            content,
            mime_type: "application/x-custom-binary".to_string(),
            metadata: Metadata::default(),
            tables: vec![],
            detected_languages: None,
            chunks: None,
        })
    }

    fn extract_sync(
        &self,
        data: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<ExtractionResult, KreuzbergError> {
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(self.extract(data, mime_type, config))
    }
}

#[tokio::main]
async fn main() -> kreuzberg::Result<()> {
    println!("=== Registering Custom CSV Extractor ===");
    let csv_extractor = Arc::new(CSVExtractor::new().with_headers(true)) as Arc<dyn DocumentExtractor>;

    let registry = get_document_extractor_registry();
    {
        let mut registry = registry.write().unwrap();
        registry.register(csv_extractor)?;
    }
    println!("✓ Registered CSV extractor with priority 100");

    println!("\n=== Extracting CSV File ===");
    let result = extract_file_sync("data.csv", None, &ExtractionConfig::default())?;
    println!("Extracted CSV as markdown table:");
    println!("{}", &result.content[..result.content.len().min(500)]);

    println!("\n=== Registering Binary Format Extractor ===");
    let binary_extractor = Arc::new(BinaryFormatExtractor) as Arc<dyn DocumentExtractor>;

    {
        let mut registry = registry.write().unwrap();
        registry.register(binary_extractor)?;
    }
    println!("✓ Registered binary format extractor");

    println!("\n=== Extracting Binary File ===");
    match extract_file(
        "custom.bin",
        Some("application/x-custom-binary"),
        &ExtractionConfig::default(),
    )
    .await
    {
        Ok(result) => {
            println!("Extracted binary file:");
            println!("{}", result.content);
        }
        Err(e) => eprintln!("Binary extraction error: {}", e),
    }

    println!("\n=== Registered Extractors ===");
    let registry = registry.read().unwrap();
    println!("Total extractors: {}", registry.len());

    Ok(())
}
