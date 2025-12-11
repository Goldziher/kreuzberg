# Rust API Reference

Complete reference for the Kreuzberg Rust API.

## Installation

Add to your `Cargo.toml`:

```toml title="Cargo.toml"
[dependencies]
kreuzberg = "4.0"
tokio = { version = "1", features = ["rt", "macros"] }
```

**With specific features:**

```toml title="Cargo.toml"
[dependencies]
kreuzberg = { version = "4.0", features = ["pdf", "ocr", "chunking", "api"] }
```

**Available features:**

- `pdf` - PDF extraction support (enabled by default)
- `ocr` - OCR support with Tesseract
- `chunking` - Text chunking algorithms
- `language-detection` - Language detection
- `keywords-yake` - YAKE keyword extraction
- `keywords-rake` - RAKE keyword extraction
- `api` - HTTP API server support
- `mcp` - Model Context Protocol server support

## Core Functions

### extract_file_sync()

Extract content from a file (synchronous, blocking).

**Signature:**

```rust title="Rust"
pub fn extract_file_sync(
    file_path: impl AsRef<Path>,
    mime_type: Option<&str>,
    config: &ExtractionConfig
) -> Result<ExtractionResult>
```

**Parameters:**

- `file_path` (impl AsRef<Path>): Path to the file to extract
- `mime_type` (Option<&str>): Optional MIME type hint. If None, MIME type is auto-detected
- `config` (&ExtractionConfig): Extraction configuration reference

**Returns:**

- `Result<ExtractionResult>`: Result containing extraction result or error

**Errors:**

- `KreuzbergError::Io` - File system errors (file not found, permission denied, etc.)
- `KreuzbergError::Validation` - Invalid configuration or file path
- `KreuzbergError::Parsing` - Document parsing failure
- `KreuzbergError::Ocr` - OCR processing failure
- `KreuzbergError::MissingDependency` - Required system dependency not found

**Examples:**

```rust title="basic_extraction.rs"
use kreuzberg::{extract_file_sync, ExtractionConfig};

fn main() -> kreuzberg::Result<()> {
    // Extract a document synchronously with default configuration
    let config = ExtractionConfig::default();
    let result = extract_file_sync("document.pdf", None, &config)?;

    println!("Content: {}", result.content);
    println!("Pages: {}", result.metadata.page_count.unwrap_or(0));

    Ok(())
}
```

```rust title="with_ocr.rs"
use kreuzberg::{extract_file_sync, ExtractionConfig, OcrConfig};

fn main() -> kreuzberg::Result<()> {
    // Configure OCR for scanned documents
    let config = ExtractionConfig {
        ocr: Some(OcrConfig::default()),
        force_ocr: false,
        ..Default::default()
    };

    let result = extract_file_sync("scanned.pdf", None, &config)?;
    println!("Extracted: {}", result.content);

    Ok(())
}
```

---

### extract_file()

Extract content from a file (asynchronous).

**Signature:**

```rust title="Rust"
pub async fn extract_file(
    file_path: impl AsRef<Path>,
    mime_type: Option<&str>,
    config: &ExtractionConfig
) -> Result<ExtractionResult>
```

**Parameters:**

Same as [`extract_file_sync()`](#extract_file_sync).

**Returns:**

- `Result<ExtractionResult>`: Result containing extraction result or error

**Examples:**

```rust title="async_extraction.rs"
use kreuzberg::{extract_file, ExtractionConfig};

#[tokio::main]
async fn main() -> kreuzberg::Result<()> {
    // Extract a document asynchronously
    let config = ExtractionConfig::default();
    let result = extract_file("document.pdf", None, &config).await?;

    println!("Content: {}", result.content);
    Ok(())
}
```

---

### extract_bytes_sync()

Extract content from bytes (synchronous, blocking).

**Signature:**

```rust title="Rust"
pub fn extract_bytes_sync(
    data: &[u8],
    mime_type: &str,
    config: &ExtractionConfig
) -> Result<ExtractionResult>
```

**Parameters:**

- `data` (&[u8]): File content as byte slice
- `mime_type` (&str): MIME type of the data (required for format detection)
- `config` (&ExtractionConfig): Extraction configuration reference

**Returns:**

- `Result<ExtractionResult>`: Result containing extraction result or error

**Examples:**

```rust title="byte_extraction.rs"
use kreuzberg::{extract_bytes_sync, ExtractionConfig};
use std::fs;

fn main() -> kreuzberg::Result<()> {
    // Extract from in-memory byte array
    let data = fs::read("document.pdf")?;
    let config = ExtractionConfig::default();
    let result = extract_bytes_sync(&data, "application/pdf", &config)?;

    println!("Content: {}", result.content);
    Ok(())
}
```

---

### extract_bytes()

Extract content from bytes (asynchronous).

**Signature:**

```rust title="Rust"
pub async fn extract_bytes(
    data: &[u8],
    mime_type: &str,
    config: &ExtractionConfig
) -> Result<ExtractionResult>
```

**Parameters:**

Same as [`extract_bytes_sync()`](#extract_bytes_sync).

**Returns:**

- `Result<ExtractionResult>`: Result containing extraction result or error

---

### batch_extract_file_sync()

Extract content from multiple files in parallel (synchronous, blocking).

**Signature:**

```rust title="Rust"
pub fn batch_extract_file_sync(
    paths: &[impl AsRef<Path>],
    mime_types: Option<&[&str]>,
    config: &ExtractionConfig
) -> Result<Vec<ExtractionResult>>
```

**Parameters:**

- `paths` (&[impl AsRef<Path>]): Slice of file paths to extract
- `mime_types` (Option<&[&str]>): Optional MIME type hints (must match paths length if provided)
- `config` (&ExtractionConfig): Extraction configuration applied to all files

**Returns:**

- `Result<Vec<ExtractionResult>>`: Result containing vector of extraction results

**Examples:**

```rust title="batch_processing.rs"
use kreuzberg::{batch_extract_file_sync, ExtractionConfig};

fn main() -> kreuzberg::Result<()> {
    // Process multiple files in parallel for better performance
    let paths = ["doc1.pdf", "doc2.docx", "doc3.xlsx"];
    let config = ExtractionConfig::default();
    let results = batch_extract_file_sync(&paths, None, &config)?;

    // Display results for each file
    for (i, result) in results.iter().enumerate() {
        println!("{}: {} characters", paths[i], result.content.len());
    }

    Ok(())
}
```

---

### batch_extract_file()

Extract content from multiple files in parallel (asynchronous).

**Signature:**

```rust title="Rust"
pub async fn batch_extract_file(
    paths: &[impl AsRef<Path>],
    mime_types: Option<&[&str]>,
    config: &ExtractionConfig
) -> Result<Vec<ExtractionResult>>
```

**Parameters:**

Same as [`batch_extract_file_sync()`](#batch_extract_file_sync).

**Returns:**

- `Result<Vec<ExtractionResult>>`: Result containing vector of extraction results

**Examples:**

```rust title="async_batch_processing.rs"
use kreuzberg::{batch_extract_file, ExtractionConfig};

#[tokio::main]
async fn main() -> kreuzberg::Result<()> {
    // Process multiple files asynchronously in parallel
    let files = ["doc1.pdf", "doc2.docx", "doc3.xlsx"];
    let config = ExtractionConfig::default();
    let results = batch_extract_file(&files, None, &config).await?;

    // Print extracted content from each file
    for result in results {
        println!("{}", result.content);
    }

    Ok(())
}
```

---

### batch_extract_bytes_sync()

Extract content from multiple byte arrays in parallel (synchronous, blocking).

**Signature:**

```rust title="Rust"
pub fn batch_extract_bytes_sync(
    data_list: &[&[u8]],
    mime_types: &[&str],
    config: &ExtractionConfig
) -> Result<Vec<ExtractionResult>>
```

**Parameters:**

- `data_list` (&[&[u8]]): Slice of file contents as byte slices
- `mime_types` (&[&str]): Slice of MIME types (must match data_list length)
- `config` (&ExtractionConfig): Extraction configuration applied to all items

**Returns:**

- `Result<Vec<ExtractionResult>>`: Result containing vector of extraction results

---

### batch_extract_bytes()

Extract content from multiple byte arrays in parallel (asynchronous).

**Signature:**

```rust title="Rust"
pub async fn batch_extract_bytes(
    data_list: &[&[u8]],
    mime_types: &[&str],
    config: &ExtractionConfig
) -> Result<Vec<ExtractionResult>>
```

**Parameters:**

Same as [`batch_extract_bytes_sync()`](#batch_extract_bytes_sync).

**Returns:**

- `Result<Vec<ExtractionResult>>`: Result containing vector of extraction results

---

## Configuration

### ExtractionConfig

Main configuration struct for extraction operations.

**Definition:**

```rust title="Rust"
#[derive(Debug, Clone, Default)]
pub struct ExtractionConfig {
    pub ocr: Option<OcrConfig>,
    pub force_ocr: bool,
    pub pdf_options: Option<PdfConfig>,
    pub chunking: Option<ChunkingConfig>,
    pub language_detection: Option<LanguageDetectionConfig>,
    pub token_reduction: Option<TokenReductionConfig>,
    pub image_extraction: Option<ImageExtractionConfig>,
    pub post_processor: Option<PostProcessorConfig>,
}
```

**Fields:**

- `ocr` (Option<OcrConfig>): OCR configuration. Default: None (no OCR)
- `force_ocr` (bool): Force OCR even for text-based PDFs. Default: false
- `pdf_options` (Option<PdfConfig>): PDF-specific configuration. Default: None
- `chunking` (Option<ChunkingConfig>): Text chunking configuration. Default: None
- `language_detection` (Option<LanguageDetectionConfig>): Language detection configuration. Default: None
- `token_reduction` (Option<TokenReductionConfig>): Token reduction configuration. Default: None
- `image_extraction` (Option<ImageExtractionConfig>): Image extraction from documents. Default: None
- `post_processor` (Option<PostProcessorConfig>): Post-processing configuration. Default: None

**Example:**

```rust title="advanced_config.rs"
use kreuzberg::{ExtractionConfig, OcrConfig, PdfConfig};

// Configure extraction with OCR and PDF-specific options
let config = ExtractionConfig {
    ocr: Some(OcrConfig::default()),
    force_ocr: false,
    pdf_options: Some(PdfConfig {
        passwords: Some(vec!["password1".to_string(), "password2".to_string()]),
        extract_images: true,
        image_dpi: 300,
    }),
    ..Default::default()
};
```

---

### OcrConfig

OCR processing configuration.

**Definition:**

```rust title="Rust"
#[derive(Debug, Clone)]
pub struct OcrConfig {
    pub backend: String,
    pub language: String,
    pub tesseract_config: Option<TesseractConfig>,
}
```

**Fields:**

- `backend` (String): OCR backend to use. Options: "tesseract". Default: "tesseract"
- `language` (String): Language code for OCR (ISO 639-3). Default: "eng"
- `tesseract_config` (Option<TesseractConfig>): Tesseract-specific configuration. Default: None

**Example:**

```rust title="ocr_config.rs"
use kreuzberg::OcrConfig;

// Configure OCR backend and language settings
let ocr_config = OcrConfig {
    backend: "tesseract".to_string(),
    language: "eng".to_string(),
    tesseract_config: None,
};
```

---

### TesseractConfig

Tesseract OCR backend configuration.

**Definition:**

```rust title="Rust"
#[derive(Debug, Clone)]
pub struct TesseractConfig {
    pub psm: i32,
    pub oem: i32,
    pub enable_table_detection: bool,
    pub tessedit_char_whitelist: Option<String>,
    pub tessedit_char_blacklist: Option<String>,
}
```

**Fields:**

- `psm` (i32): Page segmentation mode (0-13). Default: 3 (auto)
- `oem` (i32): OCR engine mode (0-3). Default: 3 (LSTM only)
- `enable_table_detection` (bool): Enable table detection and extraction. Default: false
- `tessedit_char_whitelist` (Option<String>): Character whitelist. Default: None
- `tessedit_char_blacklist` (Option<String>): Character blacklist. Default: None

**Example:**

```rust title="tesseract_config.rs"
use kreuzberg::{ExtractionConfig, OcrConfig, TesseractConfig};

// Configure Tesseract with custom settings for numeric extraction
let config = ExtractionConfig {
    ocr: Some(OcrConfig {
        backend: "tesseract".to_string(),
        language: "eng".to_string(),
        tesseract_config: Some(TesseractConfig {
            psm: 6,                                              // Assume uniform block of text
            oem: 3,                                              // LSTM neural net mode
            enable_table_detection: true,
            tessedit_char_whitelist: Some("0123456789".to_string()),  // Only recognize digits
            tessedit_char_blacklist: None,
        }),
    }),
    ..Default::default()
};
```

---

### PdfConfig

PDF-specific configuration.

**Definition:**

```rust title="Rust"
#[derive(Debug, Clone, Default)]
pub struct PdfConfig {
    pub passwords: Option<Vec<String>>,
    pub extract_images: bool,
    pub image_dpi: u32,
}
```

**Fields:**

- `passwords` (Option<Vec<String>>): List of passwords to try for encrypted PDFs. Default: None
- `extract_images` (bool): Extract images from PDF. Default: false
- `image_dpi` (u32): DPI for image extraction. Default: 300

**Example:**

```rust title="pdf_config.rs"
use kreuzberg::PdfConfig;

// Configure PDF extraction with passwords and image settings
let pdf_config = PdfConfig {
    passwords: Some(vec!["password1".to_string(), "password2".to_string()]),
    extract_images: true,
    image_dpi: 300,
};
```

---

### ChunkingConfig

Text chunking configuration for splitting long documents.

**Definition:**

```rust title="Rust"
#[derive(Debug, Clone)]
pub struct ChunkingConfig {
    pub chunk_size: usize,
    pub chunk_overlap: usize,
    pub chunking_strategy: String,
}
```

**Fields:**

- `chunk_size` (usize): Maximum chunk size in tokens. Default: 512
- `chunk_overlap` (usize): Overlap between chunks in tokens. Default: 50
- `chunking_strategy` (String): Chunking strategy. Options: "fixed", "semantic". Default: "fixed"

---

### LanguageDetectionConfig

Language detection configuration.

**Definition:**

```rust title="Rust"
#[derive(Debug, Clone)]
pub struct LanguageDetectionConfig {
    pub enabled: bool,
    pub confidence_threshold: f64,
}
```

**Fields:**

- `enabled` (bool): Enable language detection. Default: true
- `confidence_threshold` (f64): Minimum confidence threshold (0.0-1.0). Default: 0.5

---

## Results & Types

### ExtractionResult

Result struct returned by all extraction functions.

**Definition:**

```rust title="Rust"
#[derive(Debug, Clone)]
pub struct ExtractionResult {
    pub content: String,
    pub mime_type: String,
    pub metadata: Metadata,
    pub tables: Vec<Table>,
    pub detected_languages: Option<Vec<String>>,
}
```

**Fields:**

- `content` (String): Extracted text content
- `mime_type` (String): MIME type of the processed document
- `metadata` (Metadata): Document metadata (format-specific fields)
- `tables` (Vec<Table>): Vector of extracted tables
- `detected_languages` (Option<Vec<String>>): Vector of detected language codes (ISO 639-1) if language detection is enabled

**Example:**

```rust title="result_access.rs"
use kreuzberg::{extract_file_sync, ExtractionConfig};

fn main() -> kreuzberg::Result<()> {
    let config = ExtractionConfig::default();
    let result = extract_file_sync("document.pdf", None, &config)?;

    // Access extraction result fields
    println!("Content: {}", result.content);
    println!("MIME type: {}", result.mime_type);
    println!("Tables: {}", result.tables.len());

    // Display detected languages if available
    if let Some(langs) = result.detected_languages {
        println!("Languages: {}", langs.join(", "));
    }

    Ok(())
}
```

---

### Metadata

Document metadata with format-specific fields.

**Definition:**

```rust title="Rust"
#[derive(Debug, Clone, Default)]
pub struct Metadata {
    // Common fields
    pub language: Option<String>,
    pub date: Option<String>,
    pub subject: Option<String>,
    pub format_type: Option<String>,

    // PDF-specific fields
    pub title: Option<String>,
    pub author: Option<String>,
    pub page_count: Option<usize>,
    pub creation_date: Option<String>,
    pub modification_date: Option<String>,
    pub creator: Option<String>,
    pub producer: Option<String>,
    pub keywords: Option<String>,

    // Additional fields via HashMap
    pub extra: HashMap<String, serde_json::Value>,
}
```

**Example:**

```rust title="metadata_access.rs"
let result = extract_file_sync("document.pdf", None, &config)?;
let metadata = &result.metadata;

// Access PDF-specific metadata fields
if metadata.format_type.as_deref() == Some("pdf") {
    if let Some(title) = &metadata.title {
        println!("Title: {}", title);
    }
    if let Some(pages) = metadata.page_count {
        println!("Pages: {}", pages);
    }
}
```

See the Types Reference for complete metadata field documentation.

---

### Table

Extracted table structure.

**Definition:**

```rust title="Rust"
#[derive(Debug, Clone)]
pub struct Table {
    pub cells: Vec<Vec<String>>,
    pub markdown: String,
    pub page_number: usize,
}
```

**Fields:**

- `cells` (Vec<Vec<String>>): 2D vector of table cells (rows x columns)
- `markdown` (String): Table rendered as markdown
- `page_number` (usize): Page number where table was found

**Example:**

```rust title="table_processing.rs"
let result = extract_file_sync("invoice.pdf", None, &config)?;

// Process all extracted tables
for table in &result.tables {
    println!("Table on page {}:", table.page_number);
    println!("{}", table.markdown);
    println!();
}
```

---

## Error Handling

### KreuzbergError

All errors are returned as `KreuzbergError` enum.

**Definition:**

```rust title="error_handling.rs"
#[derive(Debug, thiserror::Error)]
pub enum KreuzbergError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Parsing error: {0}")]
    Parsing(String),

    #[error("OCR error: {0}")]
    Ocr(String),

    #[error("Missing dependency: {0}")]
    MissingDependency(String),

    // ... additional variants
}
```

**Error Handling:**

```rust title="error_handling.rs"
use kreuzberg::{extract_file_sync, ExtractionConfig, KreuzbergError};

fn process_file(path: &str) -> kreuzberg::Result<String> {
    let config = ExtractionConfig::default();

    // Pattern match on specific error types for custom handling
    match extract_file_sync(path, None, &config) {
        Ok(result) => Ok(result.content),
        Err(KreuzbergError::Io(e)) => {
            eprintln!("File system error: {}", e);
            Err(KreuzbergError::Io(e))
        }
        Err(KreuzbergError::Validation(msg)) => {
            eprintln!("Invalid input: {}", msg);
            Err(KreuzbergError::Validation(msg))
        }
        Err(KreuzbergError::Parsing(msg)) => {
            eprintln!("Failed to parse document: {}", msg);
            Err(KreuzbergError::Parsing(msg))
        }
        Err(e) => Err(e),
    }
}
```

**Using the `?` operator:**

```rust title="simple_error_handling.rs"
fn main() -> kreuzberg::Result<()> {
    // Use ? operator for simple error propagation
    let config = ExtractionConfig::default();
    let result = extract_file_sync("document.pdf", None, &config)?;
    println!("{}", result.content);
    Ok(())
}
```

See [Error Handling Reference](errors.md) for detailed error documentation.

---

## Plugin System

### Document Extractors

Register custom document extractors for new file formats.

**Trait:**

```rust title="Rust"
#[async_trait]
pub trait DocumentExtractor: Send + Sync {
    fn name(&self) -> &str;
    fn mime_types(&self) -> &[&str];
    fn priority(&self) -> i32;

    async fn extract(
        &self,
        data: &[u8],
        mime_type: &str,
        config: &ExtractionConfig
    ) -> Result<ExtractionResult>;
}
```

**Registration:**

```rust title="plugin_registration.rs"
use kreuzberg::plugins::registry::get_document_extractor_registry;
use std::sync::Arc;

// Register a custom document extractor for new file formats
let registry = get_document_extractor_registry();
registry.register("custom", Arc::new(MyCustomExtractor))?;
```

---

## MIME Type Detection

### detect_mime_type()

Detect MIME type from file path.

**Signature:**

```rust title="Rust"
pub fn detect_mime_type(file_path: impl AsRef<Path>) -> Result<String>
```

**Example:**

```rust title="mime_detection.rs"
use kreuzberg::detect_mime_type;

// Detect MIME type from file path
let mime_type = detect_mime_type("document.pdf")?;
println!("MIME type: {}", mime_type); // "application/pdf"
```

---

### validate_mime_type()

Validate if a MIME type is supported.

**Signature:**

```rust title="Rust"
pub fn validate_mime_type(mime_type: &str) -> bool
```

**Example:**

```rust title="mime_validation.rs"
use kreuzberg::validate_mime_type;

// Check if a MIME type is supported by Kreuzberg
if validate_mime_type("application/pdf") {
    println!("PDF is supported");
}
```

---

## Complete Documentation

For complete Rust API documentation with all types, traits, and functions:

```bash title="Terminal"
cargo doc --open --no-deps
```

Or visit [docs.rs/kreuzberg](https://docs.rs/kreuzberg)
