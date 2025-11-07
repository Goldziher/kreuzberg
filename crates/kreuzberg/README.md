# Kreuzberg (Rust Core)

[![Crates.io](https://img.shields.io/crates/v/kreuzberg)](https://crates.io/crates/kreuzberg)
[![PyPI](https://img.shields.io/pypi/v/kreuzberg)](https://pypi.org/project/kreuzberg/)
[![npm](https://img.shields.io/npm/v/@goldziher/kreuzberg)](https://www.npmjs.com/package/@goldziher/kreuzberg)
[![RubyGems](https://img.shields.io/gem/v/kreuzberg)](https://rubygems.org/gems/kreuzberg)
[![docs.rs](https://docs.rs/kreuzberg/badge.svg)](https://docs.rs/kreuzberg)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Documentation](https://img.shields.io/badge/docs-kreuzberg.dev-blue)](https://kreuzberg.dev)

High-performance document intelligence library for Rust. Extract text, metadata, and structured information from PDFs, Office documents, images, and 50+ formats.

This is the core Rust library that powers the Python and TypeScript bindings.

## Installation

```toml
[dependencies]
kreuzberg = "4.0"
tokio = { version = "1", features = ["rt", "macros"] }
```

## Quick Start

```rust
use kreuzberg::{extract_file_sync, ExtractionConfig};

fn main() -> kreuzberg::Result<()> {
    let config = ExtractionConfig::default();
    let result = extract_file_sync("document.pdf", None, &config)?;
    println!("{}", result.content);
    Ok(())
}
```

### Async Extraction

```rust
use kreuzberg::{extract_file, ExtractionConfig};

#[tokio::main]
async fn main() -> kreuzberg::Result<()> {
    let config = ExtractionConfig::default();
    let result = extract_file("document.pdf", None, &config).await?;
    println!("{}", result.content);
    Ok(())
}
```

### Batch Processing

```rust
use kreuzberg::{batch_extract_file, ExtractionConfig};

#[tokio::main]
async fn main() -> kreuzberg::Result<()> {
    let config = ExtractionConfig::default();
    let files = vec!["doc1.pdf", "doc2.pdf", "doc3.pdf"];
    let results = batch_extract_file(&files, None, &config).await?;

    for result in results {
        println!("{}", result.content);
    }
    Ok(())
}
```

## OCR with Table Extraction

```rust
use kreuzberg::{extract_file_sync, ExtractionConfig, OcrConfig, TesseractConfig};

fn main() -> kreuzberg::Result<()> {
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: "eng".to_string(),
            tesseract_config: Some(TesseractConfig {
                enable_table_detection: true,
                ..Default::default()
            }),
        }),
        ..Default::default()
    };

    let result = extract_file_sync("invoice.pdf", None, &config)?;

    for table in &result.tables {
        println!("{}", table.markdown);
    }
    Ok(())
}
```

## Password-Protected PDFs

```rust
use kreuzberg::{extract_file_sync, ExtractionConfig, PdfConfig};

fn main() -> kreuzberg::Result<()> {
    let config = ExtractionConfig {
        pdf_options: Some(PdfConfig {
            passwords: Some(vec!["password1".to_string(), "password2".to_string()]),
            ..Default::default()
        }),
        ..Default::default()
    };

    let result = extract_file_sync("protected.pdf", None, &config)?;
    Ok(())
}
```

## Extract from Bytes

```rust
use kreuzberg::{extract_bytes_sync, ExtractionConfig};
use std::fs;

fn main() -> kreuzberg::Result<()> {
    let data = fs::read("document.pdf")?;
    let config = ExtractionConfig::default();
    let result = extract_bytes_sync(&data, "application/pdf", &config)?;
    println!("{}", result.content);
    Ok(())
}
```

## Features

The crate uses feature flags for optional functionality:

```toml
[dependencies]
kreuzberg = { version = "4.0", features = ["pdf", "excel", "ocr"] }
```

### Available Features

| Feature | Description | Binary Size |
|---------|-------------|-------------|
| `pdf` | PDF extraction via pdfium | +25MB |
| `excel` | Excel/spreadsheet parsing | +3MB |
| `office` | DOCX, PPTX extraction | +1MB |
| `email` | EML, MSG extraction | +500KB |
| `html` | HTML to markdown | +1MB |
| `xml` | XML streaming parser | +500KB |
| `archives` | ZIP, TAR, 7Z extraction | +2MB |
| `ocr` | OCR with Tesseract | +5MB |
| `language-detection` | Language detection | +100KB |
| `chunking` | Text chunking | +200KB |
| `quality` | Text quality processing | +500KB |

### Feature Bundles

```toml
kreuzberg = { version = "4.0", features = ["full"] }
kreuzberg = { version = "4.0", features = ["server"] }
kreuzberg = { version = "4.0", features = ["cli"] }
```

## Documentation

**[API Documentation](https://docs.rs/kreuzberg)** – Complete API reference with examples

**[https://kreuzberg.dev](https://kreuzberg.dev)** – User guide and tutorials

## License

MIT License - see [LICENSE](../../LICENSE) for details.
