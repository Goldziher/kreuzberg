# Kreuzberg

**High-performance document intelligence library with native support for Python, TypeScript, Rust, and Ruby.**

Kreuzberg extracts text, tables, and metadata from PDFs, images, Office documents, emails, and more. Built on a blazing-fast Rust core with language-specific bindings.

## Quick Example

=== "Python"

    ```python
    from kreuzberg import extract_file_sync

    result = extract_file_sync("document.pdf")
    print(result.content)
    print(f"Found {len(result.tables)} tables")
    ```

=== "TypeScript"

    ```typescript
    import { extractFileSync } from 'kreuzberg';

    const result = extractFileSync('document.pdf');
    console.log(result.content);
    console.log(`Found ${result.tables.length} tables`);
    ```

=== "Rust"

    ```rust
    use kreuzberg::extract_file_sync;

    let result = extract_file_sync("document.pdf", None, &Default::default())?;
    println!("{}", result.content);
    println!("Found {} tables", result.tables.len());
    ```

=== "Ruby"

    ```ruby
    require 'kreuzberg'

    result = Kreuzberg.extract_file_sync('document.pdf')
    puts result.content
    puts "Found #{result.tables.length} tables"
    ```

## Key Features

- **Multi-format support**: PDFs, images, DOCX, PPTX, XLSX, emails, HTML, Markdown, and more
- **OCR built-in**: Tesseract, EasyOCR, and PaddleOCR support for scanned documents
- **Table extraction**: Automatically detect and extract tables from documents
- **Batch processing**: Process multiple files in parallel with a single API call
- **Fast and efficient**: Rust core delivers 10-50x speedup over pure Python alternatives
- **Language detection**: Automatically identify languages in extracted text
- **Text chunking**: Semantic and token-based chunking for RAG pipelines
- **Production-ready**: Docker images, API server, and CLI included

## Supported Formats

| Format | Extensions | Notes |
|--------|-----------|-------|
| **PDF** | `.pdf` | Native text + OCR for scanned pages |
| **Images** | `.png`, `.jpg`, `.tiff`, `.bmp` | Requires OCR |
| **Office** | `.docx`, `.pptx`, `.xlsx` | Modern formats |
| **Legacy Office** | `.doc`, `.ppt`, `.xls` | Requires LibreOffice |
| **Email** | `.eml`, `.msg` | Attachments supported |
| **Web** | `.html`, `.htm` | Converted to Markdown |
| **Text** | `.md`, `.txt`, `.xml`, `.json` | Direct extraction |
| **Archives** | `.zip`, `.tar`, `.tar.gz` | Recursive extraction |

## Installation

Get started in seconds:

=== "Python"

    ```bash
    pip install kreuzberg
    ```

=== "TypeScript"

    ```bash
    npm install kreuzberg
    # or
    pnpm add kreuzberg
    ```

=== "Rust"

    ```toml
    [dependencies]
    kreuzberg = "4.0"
    ```

=== "Ruby"

    ```bash
    gem install kreuzberg
    ```

See the [Installation](getting-started/installation.md) guide for detailed instructions including optional dependencies.

## What's Next?

- [**Quick Start**](getting-started/quickstart.md) - Get up and running in 5 minutes
- [**Extraction Basics**](guides/extraction.md) - Learn the core API
- [**Configuration**](guides/configuration.md) - Configure extraction behavior
- [**OCR Guide**](guides/ocr.md) - Set up optical character recognition
- [**API Reference**](reference/api-python.md) - Detailed API documentation

## Why Kreuzberg?

**Performance**: Rust core delivers exceptional speed without sacrificing ease of use.

**Simplicity**: Just 8 extraction functions to learn - `extract_file`, `extract_bytes`, `batch_extract_files`, `batch_extract_bytes` (sync and async variants).

**Multi-language**: Use the same powerful extraction engine from Python, TypeScript, Rust, or Ruby.

**Production-ready**: Battle-tested on millions of documents, with Docker images, API server, and comprehensive error handling.

## Community

- **GitHub**: [Goldziher/kreuzberg](https://github.com/Goldziher/kreuzberg)
- **Issues**: [Report bugs or request features](https://github.com/Goldziher/kreuzberg/issues)
- **Contributing**: [Contribution guide](contributing.md)
