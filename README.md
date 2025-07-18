[![MseeP.ai Security Assessment Badge](https://mseep.net/pr/goldziher-kreuzberg-badge.png)](https://mseep.ai/app/goldziher-kreuzberg)

# Kreuzberg

[![Discord](https://img.shields.io/badge/Discord-Join%20our%20community-7289da)](https://discord.gg/pXxagNK2zN)
[![PyPI version](https://badge.fury.io/py/kreuzberg.svg)](https://badge.fury.io/py/kreuzberg)
[![Documentation](https://img.shields.io/badge/docs-kreuzberg.dev-blue)](https://kreuzberg.dev/)
[![Benchmarks](https://img.shields.io/badge/benchmarks-fastest%20CPU-orange)](https://benchmarks.kreuzberg.dev/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![DeepSource](https://app.deepsource.com/gh/Goldziher/kreuzberg.svg/?label=code+coverage&show_trend=true&token=U8AW1VWWSLwVhrbtL8LmLBDN)](https://app.deepsource.com/gh/Goldziher/kreuzberg/)

**A document intelligence framework for Python.** Extract text, metadata, and structured information from diverse document formats through a unified, extensible API. Built on established open source foundations including Pandoc, PDFium, and Tesseract.

📖 **[Complete Documentation](https://kreuzberg.dev/)**

## Framework Overview

### Document Intelligence Capabilities

- **Text Extraction**: High-fidelity text extraction preserving document structure and formatting
- **Metadata Extraction**: Comprehensive metadata including author, creation date, language, and document properties
- **Format Support**: 18 document types including PDF, Microsoft Office, images, HTML, and structured data formats
- **OCR Integration**: Multiple OCR engines (Tesseract, EasyOCR, PaddleOCR) with automatic fallback
- **Table Detection**: Structured table extraction with cell-level precision via GMFT integration
- **Document Classification**: Automatic document type detection (contracts, forms, invoices, receipts, reports)

### Technical Architecture

- **Performance**: Highest throughput among Python document processing frameworks (30+ docs/second)
- **Resource Efficiency**: 71MB installation, ~360MB runtime memory footprint
- **Extensibility**: Plugin architecture for custom extractors via the Extractor base class
- **API Design**: Synchronous and asynchronous APIs with consistent interfaces
- **Type Safety**: Complete type annotations throughout the codebase

### Open Source Foundation

Kreuzberg leverages established open source technologies:

- **Pandoc**: Universal document converter for robust format support
- **PDFium**: Google's PDF rendering engine for accurate PDF processing
- **Tesseract**: Google's OCR engine for text recognition
- **Python-docx/pptx**: Native Microsoft Office format support

## Quick Start

### Extract Text with CLI

```bash
# Extract text from any file to text format
uvx kreuzberg extract document.pdf > output.txt

# With all features (OCR, table extraction, etc.)
uvx --from "kreuzberg[all]" kreuzberg extract invoice.pdf --ocr-backend tesseract --output-format text

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

```bash
# Run the REST API
docker run -p 8000:8000 goldziher/kreuzberg

# Extract via API
curl -X POST -F "file=@document.pdf" http://localhost:8000/extract
```

📖 **[Installation Guide](https://kreuzberg.dev/getting-started/installation/)** • **[CLI Documentation](https://kreuzberg.dev/cli/)** • **[API Reference](https://kreuzberg.dev/api-reference/)**

## Deployment Options

### 🤖 MCP Server (AI Integration)

**Add to Claude Desktop with one command:**

```bash
claude mcp add kreuzberg uvx -- --from "kreuzberg[all]" kreuzberg-mcp
```

**Or configure manually in `claude_desktop_config.json`:**

```json
{
  "mcpServers": {
    "kreuzberg": {
      "command": "uvx",
      "args": ["--from", "kreuzberg[all]", "kreuzberg-mcp"]
    }
  }
}
```

**MCP capabilities:**

- Extract text from PDFs, images, Office docs, and more
- Full OCR support with multiple engines
- Table extraction and metadata parsing

📖 **[MCP Documentation](https://kreuzberg.dev/user-guide/mcp-server/)**

## Supported Formats

| Category          | Formats                        |
| ----------------- | ------------------------------ |
| **Documents**     | PDF, DOCX, DOC, RTF, TXT, EPUB |
| **Images**        | JPG, PNG, TIFF, BMP, GIF, WEBP |
| **Spreadsheets**  | XLSX, XLS, CSV, ODS            |
| **Presentations** | PPTX, PPT, ODP                 |
| **Web**           | HTML, XML, MHTML               |
| **Archives**      | Support via extraction         |

## 📊 Performance Characteristics

[View comprehensive benchmarks](https://benchmarks.kreuzberg.dev/) • [Benchmark methodology](https://github.com/Goldziher/python-text-extraction-libs-benchmarks) • [**Detailed Analysis**](https://kreuzberg.dev/performance-analysis/)

### Technical Specifications

| Metric                       | Kreuzberg Sync | Kreuzberg Async | Benchmarked        |
| ---------------------------- | -------------- | --------------- | ------------------ |
| **Throughput (tiny files)**  | 31.78 files/s  | 23.94 files/s   | Highest throughput |
| **Throughput (small files)** | 8.91 files/s   | 9.31 files/s    | Highest throughput |
| **Memory footprint**         | 359.8 MB       | 395.2 MB        | Lowest usage       |
| **Installation size**        | 71 MB          | 71 MB           | Smallest size      |
| **Success rate**             | 100%           | 100%            | Perfect            |
| **Supported formats**        | 18             | 18              | Comprehensive      |

### Architecture Advantages

- **Native C extensions**: Built on PDFium and Tesseract for maximum performance
- **Async/await support**: True asynchronous processing with intelligent task scheduling
- **Memory efficiency**: Streaming architecture minimizes memory allocation
- **Process pooling**: Automatic multiprocessing for CPU-intensive operations
- **Optimized data flow**: Efficient data handling with minimal transformations

> **Benchmark details**: Tests include PDFs, Word docs, HTML, images, and spreadsheets in multiple languages (English, Hebrew, German, Chinese, Japanese, Korean) on standardized hardware.

## Documentation

### Quick Links

- [Installation Guide](https://kreuzberg.dev/getting-started/installation/) - Setup and dependencies
- [User Guide](https://kreuzberg.dev/user-guide/) - Comprehensive usage guide
- [Performance Analysis](https://kreuzberg.dev/performance-analysis/) - Detailed benchmark results
- [API Reference](https://kreuzberg.dev/api-reference/) - Complete API documentation
- [Docker Guide](https://kreuzberg.dev/user-guide/docker/) - Container deployment
- [REST API](https://kreuzberg.dev/user-guide/api-server/) - HTTP endpoints
- [CLI Guide](https://kreuzberg.dev/cli/) - Command-line usage
- [OCR Configuration](https://kreuzberg.dev/user-guide/ocr-configuration/) - OCR engine setup

## License

MIT License - see [LICENSE](LICENSE) for details.
