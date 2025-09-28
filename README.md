# Kreuzberg

[![Discord](https://img.shields.io/badge/Discord-Join%20our%20community-7289da)](https://discord.gg/pXxagNK2zN)
[![PyPI version](https://badge.fury.io/py/kreuzberg.svg)](https://badge.fury.io/py/kreuzberg)
[![Documentation](https://img.shields.io/badge/docs-kreuzberg.dev-blue)](https://kreuzberg.dev/)
[![Benchmarks](https://img.shields.io/badge/benchmarks-fastest%20CPU-orange)](https://benchmarks.kreuzberg.dev/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![DeepSource](https://app.deepsource.com/gh/Goldziher/kreuzberg.svg/?label=code+coverage&show_trend=true&token=U8AW1VWWSLwVhrbtL8LmLBDN)](https://app.deepsource.com/gh/Goldziher/kreuzberg/)

**A document intelligence framework for Python.** Extract text, metadata, and structured information from diverse document formats through a unified, extensible API. Built on established open source foundations with hybrid Rust-Python architecture for maximum performance.

📖 **[Complete Documentation](https://kreuzberg.dev/)**

## Framework Overview

### Document Intelligence Capabilities

- **Text Extraction**: High-fidelity text extraction preserving document structure and formatting
- **Image Extraction**: Extract embedded images from PDFs, presentations, HTML, and Office documents with optional OCR
- **Metadata Extraction**: Comprehensive metadata including author, creation date, language, and document properties
- **Format Support**: 20+ document types including PDF, Microsoft Office, images, HTML, and structured data formats
- **OCR Integration**: Tesseract OCR with markdown output (default) and comprehensive table extraction
- **Table Extraction**: Multiple approaches including vision-based detection and OCR-based extraction
- **Document Classification**: Automatic document type detection (contracts, forms, invoices, receipts, reports)

### Technical Architecture

- **Hybrid Implementation**: Rust-Python architecture with performance-critical operations in Rust
- **Performance**: Fastest text extraction framework in its category
- **Resource Efficiency**: Minimal installation footprint and memory usage
- **Extensibility**: Plugin architecture for custom extractors via the Extractor base class
- **API Design**: Synchronous and asynchronous APIs with consistent interfaces
- **Type Safety**: Complete type annotations throughout the codebase

### Open Source Foundation

Kreuzberg leverages established open source technologies:

- **Pandoc**: Universal document converter for robust format support
- **PDFium**: Google's PDF rendering engine for accurate PDF processing
- **Tesseract**: Google's OCR engine for text recognition
- **Rust**: Performance-critical operations implemented in Rust for maximum speed

## Quick Start

### Extract Text with CLI

```bash
# Extract text from any file to text format
uvx kreuzberg extract document.pdf > output.txt

# With all features (chunking, language detection, etc.)
uvx kreuzberg extract invoice.pdf --ocr-backend tesseract --output-format text

# Extract with rich metadata
uvx kreuzberg extract report.pdf --show-metadata --output-format json
```

### Python Usage

**Async (recommended for web apps):**

```python
from kreuzberg import extract_file

# In your async function
result = await extract_file("presentation.pptx")
print(result.content)

# Rich metadata extraction
print(f"Title: {result.metadata.title}")
print(f"Author: {result.metadata.author}")
print(f"Page count: {result.metadata.page_count}")
print(f"Created: {result.metadata.created_at}")
```

**Sync (for scripts and CLI tools):**

```python
from kreuzberg import extract_file_sync

result = extract_file_sync("report.docx")
print(result.content)

# Access rich metadata
print(f"Language: {result.metadata.language}")
print(f"Word count: {result.metadata.word_count}")
print(f"Keywords: {result.metadata.keywords}")
```

### Docker

Two optimized images available:

```bash
# Base image (API + CLI + multilingual OCR)
docker run -p 8000:8000 goldziher/kreuzberg

# Core image (+ chunking + crypto + document classification + language detection)
docker run -p 8000:8000 goldziher/kreuzberg-core:latest

# Extract via API
curl -X POST -F "file=@document.pdf" http://localhost:8000/extract
```

📖 **[Installation Guide](https://kreuzberg.dev/getting-started/installation/)** • **[CLI Documentation](https://kreuzberg.dev/cli/)** • **[API Reference](https://kreuzberg.dev/api-reference/)**

## Deployment Options

### 🤖 MCP Server (AI Integration)

**Add to Claude Desktop with one command:**

```bash
claude mcp add kreuzberg uvx kreuzberg-mcp
```

**Or configure manually in `claude_desktop_config.json`:**

```json
{
  "mcpServers": {
    "kreuzberg": {
      "command": "uvx",
      "args": ["kreuzberg-mcp"]
    }
  }
}
```

**MCP capabilities:**

- Extract text from PDFs, images, Office docs, and more
- Multilingual OCR support with Tesseract
- Metadata parsing and language detection

📖 **[MCP Documentation](https://kreuzberg.dev/user-guide/mcp-server/)**

## Supported Formats

| Category            | Formats                        |
| ------------------- | ------------------------------ |
| **Documents**       | PDF, DOCX, DOC, RTF, TXT, EPUB |
| **Images**          | JPG, PNG, TIFF, BMP, GIF, WEBP |
| **Spreadsheets**    | XLSX, XLS, CSV, ODS            |
| **Presentations**   | PPTX, PPT, ODP                 |
| **Web**             | HTML, XML, MHTML               |
| **Structured Data** | JSON, YAML, TOML               |
| **Archives**        | Support via extraction         |

## 📊 Performance

Kreuzberg consistently ranks as the fastest Python CPU-based text extraction framework with optimal resource efficiency and 100% reliability across all tested file formats.

**[View Live Benchmarks](https://benchmarks.kreuzberg.dev/)** • **[Benchmark Methodology](https://github.com/Goldziher/python-text-extraction-libs-benchmarks)**

### Architecture Advantages

- **Hybrid Rust-Python**: Performance-critical operations in Rust for maximum speed
- **Async/await support**: True asynchronous processing with adaptive task scheduling
- **Memory efficiency**: Minimal memory allocation and optimized data handling
- **Process pooling**: Automatic multiprocessing for CPU-intensive operations
- **Native foundations**: Built on PDFium and Tesseract for proven reliability

## Documentation

### Quick Links

- [Installation Guide](https://kreuzberg.dev/getting-started/installation/) - Setup and dependencies
- [User Guide](https://kreuzberg.dev/user-guide/) - Comprehensive usage guide
- [Performance Guide](https://kreuzberg.dev/advanced/performance/) - Optimization and analysis
- [API Reference](https://kreuzberg.dev/api-reference/) - Complete API documentation
- [Docker Guide](https://kreuzberg.dev/user-guide/docker/) - Container deployment
- [REST API](https://kreuzberg.dev/user-guide/api-server/) - HTTP endpoints
- [CLI Guide](https://kreuzberg.dev/cli/) - Command-line usage
- [OCR Backends](https://kreuzberg.dev/user-guide/ocr-backends/) - OCR engine setup
- [Table Extraction](https://kreuzberg.dev/user-guide/table-extraction/) - Vision-based and OCR table extraction
- [Changelog](https://kreuzberg.dev/CHANGELOG/) - Version history and release notes

## License

MIT License - see [LICENSE](LICENSE) for details.
