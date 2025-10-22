# Rust Custom Extractor Development

Document Extractors are specialized plugins that extract text from specific file formats. This guide covers implementing custom extractors in Rust.

## Overview

Custom Document Extractors allow you to:
- Add support for new file formats
- Optimize extraction for specific document types
- Replace built-in extractors with custom logic
- Handle proprietary or domain-specific formats

**Note**: Document Extractors are only available in Rust for performance and memory safety reasons.

## Basic DocumentExtractor

### Minimal Implementation

```rust
use kreuzberg::plugins::extractor::DocumentExtractor;
use kreuzberg::types::{ExtractionResult, ExtractionConfig};
use kreuzberg::error::KreuzbergError;
use async_trait::async_trait;
use std::collections::HashMap;

pub struct SimpleExtractor;

#[async_trait]
impl DocumentExtractor for SimpleExtractor {
    async fn extract(
        &self,
        bytes: &[u8],
        _mime_type: &str,
        _config: &ExtractionConfig,
    ) -> Result<ExtractionResult, KreuzbergError> {
        // Extract text from bytes
        let text = String::from_utf8(bytes.to_vec())
            .map_err(|e| KreuzbergError::Parsing(format!("Invalid UTF-8: {}", e)))?;

        Ok(ExtractionResult {
            content: text,
            metadata: HashMap::new(),
            tables: Vec::new(),
            ..Default::default()
        })
    }

    fn supported_mime_types(&self) -> Vec<String> {
        vec!["text/plain".to_string()]
    }

    fn priority(&self) -> u32 {
        100
    }
}

// Register the extractor
use kreuzberg::plugins::registry::get_document_extractor_registry;
use std::sync::Arc;

fn register() {
    let registry = get_document_extractor_registry();
    registry.register("simple", Arc::new(SimpleExtractor)).unwrap();
}
```

### Key Requirements

1. **`extract()` method**: Async method that processes bytes and returns `ExtractionResult`
2. **`supported_mime_types()` method**: Returns list of MIME types this extractor handles
3. **`priority()` method**: Returns priority (higher = preferred, default 100)
4. **Thread-safe**: Implement `Send + Sync` traits
5. **Registration**: Register with `Arc<dyn DocumentExtractor>`

## Complete Example: CSV Extractor

```rust
use kreuzberg::plugins::extractor::DocumentExtractor;
use kreuzberg::types::{ExtractionResult, ExtractionConfig, ExtractedTable};
use kreuzberg::error::KreuzbergError;
use async_trait::async_trait;
use std::collections::HashMap;
use csv::ReaderBuilder;

pub struct CSVExtractor {
    delimiter: u8,
    has_headers: bool,
}

impl CSVExtractor {
    pub fn new() -> Self {
        Self {
            delimiter: b',',
            has_headers: true,
        }
    }

    pub fn with_delimiter(mut self, delimiter: u8) -> Self {
        self.delimiter = delimiter;
        self
    }

    pub fn with_headers(mut self, has_headers: bool) -> Self {
        self.has_headers = has_headers;
        self
    }
}

#[async_trait]
impl DocumentExtractor for CSVExtractor {
    async fn extract(
        &self,
        bytes: &[u8],
        _mime_type: &str,
        _config: &ExtractionConfig,
    ) -> Result<ExtractionResult, KreuzbergError> {
        // Parse CSV
        let mut reader = ReaderBuilder::new()
            .delimiter(self.delimiter)
            .has_headers(self.has_headers)
            .from_reader(bytes);

        // Extract table data
        let mut cells = Vec::new();
        let mut row_count = 0;
        let mut col_count = 0;

        // Add headers if present
        if self.has_headers {
            let headers: Vec<String> = reader.headers()
                .map_err(|e| KreuzbergError::Parsing(format!("CSV header error: {}", e)))?
                .iter()
                .map(|s| s.to_string())
                .collect();

            col_count = headers.len();
            cells.push(headers);
            row_count += 1;
        }

        // Add data rows
        for result in reader.records() {
            let record = result
                .map_err(|e| KreuzbergError::Parsing(format!("CSV record error: {}", e)))?;

            let row: Vec<String> = record.iter().map(|s| s.to_string()).collect();

            if col_count == 0 {
                col_count = row.len();
            }

            cells.push(row);
            row_count += 1;
        }

        // Create table
        let table = ExtractedTable {
            cells: cells.clone(),
            markdown: Some(self.cells_to_markdown(&cells)),
        };

        // Generate text representation
        let content = cells
            .iter()
            .map(|row| row.join(", "))
            .collect::<Vec<_>>()
            .join("\n");

        // Metadata
        let mut metadata = HashMap::new();
        metadata.insert("row_count".to_string(), serde_json::json!(row_count));
        metadata.insert("col_count".to_string(), serde_json::json!(col_count));
        metadata.insert("format".to_string(), serde_json::json!("csv"));

        Ok(ExtractionResult {
            content,
            metadata,
            tables: vec![table],
            ..Default::default()
        })
    }

    fn supported_mime_types(&self) -> Vec<String> {
        vec![
            "text/csv".to_string(),
            "application/csv".to_string(),
            "text/comma-separated-values".to_string(),
        ]
    }

    fn priority(&self) -> u32 {
        100
    }
}

impl CSVExtractor {
    fn cells_to_markdown(&self, cells: &[Vec<String>]) -> String {
        if cells.is_empty() {
            return String::new();
        }

        let mut md = String::new();

        // First row (header)
        md.push_str("| ");
        md.push_str(&cells[0].join(" | "));
        md.push_str(" |\n");

        // Separator
        md.push_str("| ");
        md.push_str(&vec!["---"; cells[0].len()].join(" | "));
        md.push_str(" |\n");

        // Data rows
        for row in &cells[1..] {
            md.push_str("| ");
            md.push_str(&row.join(" | "));
            md.push_str(" |\n");
        }

        md
    }
}

// Register
fn register_csv() {
    let registry = get_document_extractor_registry();
    registry.register("csv", Arc::new(CSVExtractor::new())).unwrap();
}
```

## Advanced Examples

### Binary Format Extractor

```rust
use kreuzberg::plugins::extractor::DocumentExtractor;
use kreuzberg::types::{ExtractionResult, ExtractionConfig};
use kreuzberg::error::KreuzbergError;
use async_trait::async_trait;
use std::collections::HashMap;
use byteorder::{LittleEndian, ReadBytesExt};
use std::io::Cursor;

pub struct BinaryFormatExtractor;

#[async_trait]
impl DocumentExtractor for BinaryFormatExtractor {
    async fn extract(
        &self,
        bytes: &[u8],
        _mime_type: &str,
        _config: &ExtractionConfig,
    ) -> Result<ExtractionResult, KreuzbergError> {
        // Validate magic bytes
        if bytes.len() < 4 || &bytes[0..4] != b"MYMT" {
            return Err(KreuzbergError::Parsing(
                "Invalid magic bytes - not a MYMT file".to_string()
            ));
        }

        let mut cursor = Cursor::new(&bytes[4..]);

        // Read header
        let version = cursor.read_u32::<LittleEndian>()
            .map_err(|e| KreuzbergError::Parsing(format!("Failed to read version: {}", e)))?;

        let text_length = cursor.read_u32::<LittleEndian>()
            .map_err(|e| KreuzbergError::Parsing(format!("Failed to read length: {}", e)))?;

        // Read text
        let text_start = cursor.position() as usize;
        let text_end = text_start + text_length as usize;

        if text_end > bytes.len() {
            return Err(KreuzbergError::Parsing(
                "Invalid text length - extends beyond file".to_string()
            ));
        }

        let text = String::from_utf8(bytes[text_start..text_end].to_vec())
            .map_err(|e| KreuzbergError::Parsing(format!("Invalid UTF-8 text: {}", e)))?;

        // Metadata
        let mut metadata = HashMap::new();
        metadata.insert("format_version".to_string(), serde_json::json!(version));
        metadata.insert("format".to_string(), serde_json::json!("MYMT"));

        Ok(ExtractionResult {
            content: text,
            metadata,
            tables: Vec::new(),
            ..Default::default()
        })
    }

    fn supported_mime_types(&self) -> Vec<String> {
        vec!["application/x-mymt".to_string()]
    }

    fn priority(&self) -> u32 {
        100
    }
}
```

### Streaming Parser for Large Files

```rust
use kreuzberg::plugins::extractor::DocumentExtractor;
use kreuzberg::types::{ExtractionResult, ExtractionConfig};
use kreuzberg::error::KreuzbergError;
use async_trait::async_trait;
use std::collections::HashMap;
use std::io::{BufRead, BufReader};

pub struct StreamingTextExtractor {
    max_size: usize,
}

impl StreamingTextExtractor {
    pub fn new(max_size: usize) -> Self {
        Self { max_size }
    }
}

#[async_trait]
impl DocumentExtractor for StreamingTextExtractor {
    async fn extract(
        &self,
        bytes: &[u8],
        _mime_type: &str,
        _config: &ExtractionConfig,
    ) -> Result<ExtractionResult, KreuzbergError> {
        let mut content = String::new();
        let mut line_count = 0;
        let mut word_count = 0;

        // Stream through lines
        let reader = BufReader::new(bytes);
        for line_result in reader.lines() {
            let line = line_result
                .map_err(|e| KreuzbergError::Parsing(format!("Line read error: {}", e)))?;

            // Check size limit
            if content.len() + line.len() > self.max_size {
                return Err(KreuzbergError::Parsing(
                    format!("File too large: exceeds {} bytes", self.max_size)
                ));
            }

            // Count words
            word_count += line.split_whitespace().count();
            line_count += 1;

            content.push_str(&line);
            content.push('\n');
        }

        // Metadata
        let mut metadata = HashMap::new();
        metadata.insert("line_count".to_string(), serde_json::json!(line_count));
        metadata.insert("word_count".to_string(), serde_json::json!(word_count));
        metadata.insert("char_count".to_string(), serde_json::json!(content.len()));

        Ok(ExtractionResult {
            content,
            metadata,
            tables: Vec::new(),
            ..Default::default()
        })
    }

    fn supported_mime_types(&self) -> Vec<String> {
        vec!["text/plain".to_string()]
    }

    fn priority(&self) -> u32 {
        110 // Higher priority than default text extractor
    }
}
```

### XML Parser with Namespaces

```rust
use kreuzberg::plugins::extractor::DocumentExtractor;
use kreuzberg::types::{ExtractionResult, ExtractionConfig};
use kreuzberg::error::KreuzbergError;
use async_trait::async_trait;
use std::collections::HashMap;
use quick_xml::Reader;
use quick_xml::events::Event;

pub struct XMLExtractor {
    preserve_structure: bool,
}

impl XMLExtractor {
    pub fn new(preserve_structure: bool) -> Self {
        Self { preserve_structure }
    }
}

#[async_trait]
impl DocumentExtractor for XMLExtractor {
    async fn extract(
        &self,
        bytes: &[u8],
        _mime_type: &str,
        _config: &ExtractionConfig,
    ) -> Result<ExtractionResult, KreuzbergError> {
        let mut reader = Reader::from_reader(bytes);
        reader.trim_text(true);

        let mut content = String::new();
        let mut element_count = 0;
        let mut depth = 0;
        let mut buf = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    element_count += 1;
                    depth += 1;

                    if self.preserve_structure {
                        let indent = "  ".repeat(depth - 1);
                        let name = String::from_utf8_lossy(e.name().as_ref());
                        content.push_str(&format!("{}<{}>\n", indent, name));
                    }
                }
                Ok(Event::End(e)) => {
                    depth -= 1;

                    if self.preserve_structure {
                        let indent = "  ".repeat(depth);
                        let name = String::from_utf8_lossy(e.name().as_ref());
                        content.push_str(&format!("{}</{}>\n", indent, name));
                    }
                }
                Ok(Event::Text(e)) => {
                    let text = e.unescape()
                        .map_err(|e| KreuzbergError::Parsing(format!("XML unescape error: {}", e)))?;

                    let text = text.trim();
                    if !text.is_empty() {
                        if self.preserve_structure {
                            let indent = "  ".repeat(depth);
                            content.push_str(&format!("{}{}\n", indent, text));
                        } else {
                            content.push_str(text);
                            content.push(' ');
                        }
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(KreuzbergError::Parsing(
                    format!("XML parsing error at position {}: {}", reader.buffer_position(), e)
                )),
                _ => {}
            }

            buf.clear();
        }

        // Metadata
        let mut metadata = HashMap::new();
        metadata.insert("element_count".to_string(), serde_json::json!(element_count));
        metadata.insert("format".to_string(), serde_json::json!("xml"));

        Ok(ExtractionResult {
            content: content.trim().to_string(),
            metadata,
            tables: Vec::new(),
            ..Default::default()
        })
    }

    fn supported_mime_types(&self) -> Vec<String> {
        vec![
            "application/xml".to_string(),
            "text/xml".to_string(),
        ]
    }

    fn priority(&self) -> u32 {
        100
    }
}
```

## Priority System

Multiple extractors can support the same MIME type. The priority system determines which extractor to use:

```rust
// Lower priority (default)
pub struct BasicPDFExtractor;

impl DocumentExtractor for BasicPDFExtractor {
    fn priority(&self) -> u32 {
        50 // Lower priority
    }

    fn supported_mime_types(&self) -> Vec<String> {
        vec!["application/pdf".to_string()]
    }
}

// Higher priority (optimized)
pub struct OptimizedPDFExtractor;

impl DocumentExtractor for OptimizedPDFExtractor {
    fn priority(&self) -> u32 {
        150 // Higher priority - will be used instead
    }

    fn supported_mime_types(&self) -> Vec<String> {
        vec!["application/pdf".to_string()]
    }
}
```

## Configuration Support

Access extraction configuration:

```rust
#[async_trait]
impl DocumentExtractor for ConfigurableExtractor {
    async fn extract(
        &self,
        bytes: &[u8],
        _mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<ExtractionResult, KreuzbergError> {
        // Check if OCR is enabled
        let use_ocr = config.ocr.is_some();

        // Check quality processing
        let quality_enabled = config.enable_quality_processing;

        // Your extraction logic based on config
        let content = if use_ocr {
            self.extract_with_ocr(bytes, config)?
        } else {
            self.extract_text(bytes)?
        };

        Ok(ExtractionResult {
            content,
            ..Default::default()
        })
    }
}
```

## Error Handling

```rust
use kreuzberg::error::KreuzbergError;

#[async_trait]
impl DocumentExtractor for RobustExtractor {
    async fn extract(
        &self,
        bytes: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<ExtractionResult, KreuzbergError> {
        // Validate input
        if bytes.is_empty() {
            return Err(KreuzbergError::Validation(
                "Empty input bytes".to_string()
            ));
        }

        // Try extraction
        match self.try_extract(bytes) {
            Ok(text) => {
                Ok(ExtractionResult {
                    content: text,
                    ..Default::default()
                })
            }
            Err(e) => {
                // Log error but provide context
                eprintln!("Extraction failed for {}: {}", mime_type, e);

                Err(KreuzbergError::Parsing(
                    format!("Failed to parse {}: {}", mime_type, e)
                ))
            }
        }
    }
}
```

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    #[tokio::test]
    async fn test_csv_extractor_basic() {
        let extractor = CSVExtractor::new();
        let csv_data = b"Name,Age,City\nAlice,30,NYC\nBob,25,LA";

        let config = ExtractionConfig::default();
        let result = extractor.extract(csv_data, "text/csv", &config)
            .await
            .unwrap();

        assert!(!result.content.is_empty());
        assert_eq!(result.tables.len(), 1);
        assert_eq!(result.tables[0].cells.len(), 3); // Header + 2 rows
    }

    #[tokio::test]
    async fn test_csv_extractor_invalid_utf8() {
        let extractor = CSVExtractor::new();
        let invalid_data = b"\xFF\xFE";

        let config = ExtractionConfig::default();
        let result = extractor.extract(invalid_data, "text/csv", &config).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_priority_system() {
        let basic = BasicPDFExtractor;
        let optimized = OptimizedPDFExtractor;

        assert!(optimized.priority() > basic.priority());
    }
}
```

## Best Practices

### 1. Memory Efficiency

```rust
// ✅ Good - stream large files
use tokio::io::AsyncBufReadExt;

#[async_trait]
impl DocumentExtractor for StreamingExtractor {
    async fn extract(
        &self,
        bytes: &[u8],
        _mime_type: &str,
        _config: &ExtractionConfig,
    ) -> Result<ExtractionResult, KreuzbergError> {
        // Process line by line instead of loading everything
        let reader = BufReader::new(bytes);
        // ... streaming logic
    }
}
```

### 2. Error Context

```rust
// ✅ Good - include context in errors
return Err(KreuzbergError::Parsing(
    format!("Failed to parse CSV at row {}: {}", row_num, e)
));

// ❌ Bad - no context
return Err(KreuzbergError::Parsing(e.to_string()));
```

### 3. Safety Comments

```rust
// SAFETY: Buffer is guaranteed to be valid UTF-8 by validation above ~keep
let text = unsafe { std::str::from_utf8_unchecked(buffer) };
```

### 4. Resource Cleanup

```rust
impl Drop for MyExtractor {
    fn drop(&mut self) {
        // Cleanup resources
        self.cleanup_temp_files();
    }
}
```

## Registration

```rust
use kreuzberg::plugins::registry::get_document_extractor_registry;
use std::sync::Arc;

pub fn register_extractors() {
    let registry = get_document_extractor_registry();

    // Register single extractor
    registry.register("csv", Arc::new(CSVExtractor::new()))
        .expect("Failed to register CSV extractor");

    // Register with builder pattern
    registry.register("tsv", Arc::new(
        CSVExtractor::new()
            .with_delimiter(b'\t')
            .with_headers(true)
    )).expect("Failed to register TSV extractor");
}
```

## Next Steps

- [Plugin Development Overview](overview.md) - Compare plugin types
- [Extractor Concepts](../concepts/extractors.md) - Extraction architecture
- [API Reference](../api-reference/python/) - Complete API documentation
