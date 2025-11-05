# Kreuzberg

[![Discord](https://img.shields.io/badge/Discord-Join%20our%20community-7289da)](https://discord.gg/pXxagNK2zN)
[![PyPI](https://badge.fury.io/py/kreuzberg.svg)](https://badge.fury.io/py/kreuzberg)
[![npm](https://img.shields.io/npm/v/@goldziher/kreuzberg)](https://www.npmjs.com/package/@goldziher/kreuzberg)
[![Crates.io](https://img.shields.io/crates/v/kreuzberg)](https://crates.io/crates/kreuzberg)
[![Documentation](https://img.shields.io/badge/docs-kreuzberg.dev-blue)](https://kreuzberg.dev/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

**A multiplatform document intelligence engine written in Rust.** Extract text, metadata, and structured information from PDFs, Office documents, images, and 50+ formats through a unified API. Use in Rust, Python, TypeScript/Node.js—or via CLI, REST API, or MCP.

## Why Kreuzberg

- **High-performance Rust core** – 10-50x faster than pure-Python alternatives
- **Multiplatform** – Native libraries for Rust, Python, TypeScript/Node.js
- **Production-ready** – Battle-tested with comprehensive error handling and validation
- **50+ formats** – PDF, DOCX, XLSX, PPTX, images, HTML, XML, emails, and more
- **OCR built-in** – Multiple backends (Tesseract, EasyOCR, PaddleOCR) with table extraction
- **Flexible deployment** – Use as library, CLI tool, REST API server, or MCP server
- **Memory efficient** – Streaming parsers handle multi-GB files with constant memory

📖 **[Complete Documentation](https://kreuzberg.dev/)** • 🚀 **[Quick Start Guides](#quick-start)** • 📊 **[Benchmarks](https://benchmarks.kreuzberg.dev/)**

## Installation

### CLI

```bash
brew install goldziher/tap/kreuzberg
```

```bash
cargo install kreuzberg-cli
```

### Python

```bash
pip install kreuzberg
```

### TypeScript/Node.js

```bash
npm install @goldziher/kreuzberg
```

### Rust

```toml
[dependencies]
kreuzberg = "4.0"
```

## Quick Start

### CLI

```bash
kreuzberg extract document.pdf
kreuzberg extract scanned.pdf --ocr true
kreuzberg batch *.pdf --output-format json
```

### Python

```python
from kreuzberg import extract_file_sync

result = extract_file_sync("document.pdf")
print(result.content)
print(f"Pages: {result.metadata['page_count']}")
```

### TypeScript/Node.js

```typescript
import { extractFileSync } from '@goldziher/kreuzberg';

const result = extractFileSync('document.pdf');
console.log(result.content);
console.log(`Pages: ${result.metadata.pageCount}`);
```

### Rust

```rust
use kreuzberg::{extract_file_sync, ExtractionConfig};

fn main() -> kreuzberg::Result<()> {
    let config = ExtractionConfig::default();
    let result = extract_file_sync("document.pdf", None, &config)?;
    println!("Content: {}", result.content);
    Ok(())
}
```

## Language-Specific Documentation

Each platform has detailed documentation with language-specific examples and best practices:

- **[Python Documentation](packages/python/README.md)** – Installation, examples, configuration
- **[TypeScript/Node.js Documentation](packages/typescript/README.md)** – Installation, examples, types
- **[Rust Documentation](crates/kreuzberg/README.md)** – Crate usage, features, examples

## Supported Formats

### Documents & Productivity

| Format | Extensions | Metadata | Tables | Images |
|--------|-----------|----------|--------|--------|
| PDF | `.pdf` | ✅ | ✅ | ✅ |
| Word | `.docx`, `.doc` | ✅ | ✅ | ✅ |
| Excel | `.xlsx`, `.xls`, `.ods` | ✅ | ✅ | ❌ |
| PowerPoint | `.pptx`, `.ppt` | ✅ | ✅ | ✅ |
| Rich Text | `.rtf` | ✅ | ❌ | ❌ |
| EPUB | `.epub` | ✅ | ❌ | ❌ |

### Images

All image formats support OCR: `.jpg`, `.jpeg`, `.png`, `.tiff`, `.tif`, `.bmp`, `.gif`, `.webp`, `.jp2`

### Web & Structured Data

| Format | Extensions | Features |
|--------|-----------|----------|
| HTML | `.html`, `.htm` | Metadata extraction, link preservation |
| XML | `.xml` | Streaming parser for multi-GB files |
| JSON | `.json` | Intelligent field detection |
| YAML | `.yaml`, `.yml` | Structure preservation |
| TOML | `.toml` | Configuration parsing |

### Email & Archives

| Format | Extensions | Features |
|--------|-----------|----------|
| Email | `.eml`, `.msg` | Full metadata, attachment extraction |
| Archives | `.zip`, `.tar`, `.gz`, `.7z` | File listing, metadata |

### Academic & Technical

LaTeX (`.tex`), BibTeX (`.bib`), Jupyter (`.ipynb`), reStructuredText (`.rst`), Org Mode (`.org`), Markdown (`.md`)

**[Complete Format Documentation](https://kreuzberg.dev/formats/)**

## Key Features

### OCR with Table Extraction

```python
from kreuzberg import extract_file_sync, ExtractionConfig, OcrConfig, TesseractConfig

config = ExtractionConfig(
    ocr=OcrConfig(
        backend="tesseract",
        tesseract_config=TesseractConfig(
            enable_table_detection=True
        )
    )
)

result = extract_file_sync("scanned_invoice.pdf", config=config)
for table in result.tables:
    print(table.markdown)
```

### Batch Processing

```typescript
import { batchExtractFiles } from '@goldziher/kreuzberg';

const files = ['doc1.pdf', 'doc2.docx', 'doc3.xlsx'];
const results = await batchExtractFiles(files);

for (const result of results) {
  console.log(result.content);
}
```

### Password-Protected PDFs

```rust
use kreuzberg::{extract_file_sync, ExtractionConfig, PdfConfig};

let config = ExtractionConfig {
    pdf_options: Some(PdfConfig {
        passwords: Some(vec!["password1".to_string(), "password2".to_string()]),
        ..Default::default()
    }),
    ..Default::default()
};

let result = extract_file_sync("protected.pdf", None, &config)?;
```

### Language Detection

```python
from kreuzberg import extract_file_sync, ExtractionConfig, LanguageDetectionConfig

config = ExtractionConfig(
    language_detection=LanguageDetectionConfig(enabled=True)
)

result = extract_file_sync("multilingual.pdf", config=config)
print(result.detected_languages)
```

### Metadata Extraction

```typescript
import { extractFileSync } from '@goldziher/kreuzberg';

const result = extractFileSync('document.pdf');

console.log(result.metadata.pdf?.title);
console.log(result.metadata.pdf?.author);
console.log(result.metadata.pdf?.pageCount);
console.log(result.metadata.pdf?.creationDate);
```

## Deployment Options

### REST API Server

```bash
uvx kreuzberg serve --port 8000
```

```bash
curl -X POST -F "file=@document.pdf" http://localhost:8000/extract
```

**[API Documentation](https://kreuzberg.dev/user-guide/api-server/)**

### MCP Server (AI Integration)

```bash
claude mcp add kreuzberg uvx kreuzberg-mcp
```

**[MCP Documentation](https://kreuzberg.dev/user-guide/mcp-server/)**

### Docker

Kreuzberg provides official Docker images with two variants:

**Core** (~1.0-1.3GB): Production-ready with Tesseract OCR, Pandoc, modern Office formats
```bash
docker pull goldziher/kreuzberg:v4-core
docker run -p 8000:8000 goldziher/kreuzberg:v4-core
```

**Full** (~1.5-2.1GB): Adds LibreOffice for legacy Office formats (.doc, .ppt)
```bash
docker pull goldziher/kreuzberg:v4-full
docker run -p 8000:8000 goldziher/kreuzberg:v4-full
```

Each image supports three execution modes:

```bash
# API Server (default)
docker run -p 8000:8000 goldziher/kreuzberg:v4-core

# CLI Mode
docker run -v $(pwd):/data goldziher/kreuzberg:v4-core extract /data/document.pdf

# MCP Server
docker run goldziher/kreuzberg:v4-core mcp
```

**[Docker Deployment Guide](https://kreuzberg.dev/guides/docker/)**

## Performance

Kreuzberg consistently ranks as the **fastest CPU-based document extraction framework**, with optimal resource efficiency and 100% reliability across tested formats.

**[View Live Benchmarks](https://benchmarks.kreuzberg.dev/)** • **[Benchmark Methodology](https://github.com/Goldziher/python-text-extraction-libs-benchmarks)**

### Architecture Advantages

- **Rust core** – Performance-critical operations in native code
- **Async throughout** – True asynchronous processing with Tokio runtime
- **Memory efficient** – Streaming parsers for large files
- **Parallel batch processing** – Configurable concurrency
- **Zero-copy operations** – Efficient data handling where possible

## Documentation

- **[Installation Guide](https://kreuzberg.dev/getting-started/installation/)** – Setup and dependencies
- **[User Guide](https://kreuzberg.dev/user-guide/)** – Comprehensive usage guide
- **[API Reference](https://kreuzberg.dev/api-reference/)** – Complete API documentation
- **[Format Support](https://kreuzberg.dev/formats/)** – Supported file formats
- **[OCR Backends](https://kreuzberg.dev/user-guide/ocr-backends/)** – OCR engine setup
- **[CLI Guide](https://kreuzberg.dev/cli/)** – Command-line usage
- **[Migration Guide](https://kreuzberg.dev/migration/v3-to-v4/)** – Upgrading from v3

## Contributing

Contributions are welcome! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

MIT License - see [LICENSE](LICENSE) for details.
