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
- **Format Support**: 50+ document types including PDF, Microsoft Office (modern + legacy), images, HTML, XML, and structured data formats
- **OCR Integration**: Multiple OCR backends with different strengths (Tesseract, EasyOCR, PaddleOCR)
- **Table Extraction**: Multiple approaches including vision-based detection and OCR-based extraction

### OCR Backends

| Backend       | Best For                         | Model Size | Installation                         |
| ------------- | -------------------------------- | ---------- | ------------------------------------ |
| **Tesseract** | Printed text, CPU                | 5-10MB     | System package (default)             |
| **EasyOCR**   | Scene text, GPU, handwriting     | 100-500MB  | `pip install "kreuzberg[easyocr]"`   |
| **PaddleOCR** | Complex layouts, Asian languages | 10-50MB    | `pip install "kreuzberg[paddleocr]"` |

```python
from kreuzberg import extract_file, ExtractionConfig, EasyOCRConfig, PaddleOCRConfig

# Tesseract (default)
result = await extract_file("document.pdf")

# EasyOCR (GPU-accelerated)
result = await extract_file("photo.jpg", config=ExtractionConfig(ocr=EasyOCRConfig(language=("en",), device="cuda")))

# PaddleOCR (complex layouts)
result = await extract_file("invoice.pdf", config=ExtractionConfig(ocr=PaddleOCRConfig(language="ch")))
```

📖 **[OCR Configuration Guide](https://kreuzberg.dev/user-guide/ocr-backends/)**

- **Document Classification**: Automatic document type detection (contracts, forms, invoices, receipts, reports)
- **Streaming Parsers**: Memory-efficient Rust streaming for XML, plain text, and markdown (handles multi-GB files)

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

### Advanced Examples

**XML extraction with streaming parser:**

```python
from kreuzberg import extract_file_sync

# Handles multi-GB XML files efficiently
result = extract_file_sync("large_dataset.xml")
print(f"Element count: {result.metadata['element_count']}")
print(f"Unique elements: {result.metadata['unique_elements']}")
```

**Legacy Office formats (.doc, .ppt):**

```python
from kreuzberg import extract_file_sync

# Requires LibreOffice installed
result = extract_file_sync("legacy_document.doc")
print(result.content)

result = extract_file_sync("old_presentation.ppt")
print(f"Slides: {result.metadata['slide_count']}")
```

**Markdown with metadata extraction:**

```python
from kreuzberg import extract_file_sync

result = extract_file_sync("README.md")
print(f"Headers: {result.metadata['headers']}")  # All markdown headers
print(f"Links: {result.metadata['links']}")  # All [text](url) links
print(f"Code blocks: {result.metadata['code_blocks']}")  # Language and code
```

### Docker

Two optimized images available:

```bash
# Base image (API + CLI + multilingual OCR)
docker run -p 8000:8000 goldziher/kreuzberg

# Core image (+ chunking + crypto + document classification + language detection)
docker run -p 8000:8000 goldziher/kreuzberg-core:latest

# Extract via API
curl -X POST -F "files=@document.pdf" http://localhost:8000/extract
```

📖 **[Installation Guide](https://kreuzberg.dev/getting-started/installation/)** • **[CLI Documentation](https://kreuzberg.dev/cli/)** • **[API Reference](https://kreuzberg.dev/api-reference/)**

### System Dependencies

#### LibreOffice (Optional - for legacy Office formats)

Required only for `.doc` and `.ppt` file support. Modern Office formats (`.docx`, `.pptx`, `.xlsx`) work without LibreOffice.

**macOS:**

```bash
brew install libreoffice
```

**Ubuntu/Debian:**

```bash
sudo apt-get update
sudo apt-get install libreoffice
```

**RHEL/CentOS/Fedora:**

```bash
sudo dnf install libreoffice
```

**Windows:**

Download from [libreoffice.org](https://www.libreoffice.org/download/download/) and add to PATH.

**Docker:**

LibreOffice is pre-installed in all Kreuzberg Docker images.

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

### Document Formats

| Format            | Extensions | Implementation | OCR | Table Extraction | Metadata    | Notes                                 |
| ----------------- | ---------- | -------------- | --- | ---------------- | ----------- | ------------------------------------- |
| **PDF**           | `.pdf`     | PDFium (Rust)  | ✅  | ✅ Vision-based  | ✅ Full     | Fastest, most reliable PDF extraction |
| **Word (Modern)** | `.docx`    | Pandoc         | ❌  | ✅ Native        | ✅ Full     | Office Open XML format                |
| **Word (Legacy)** | `.doc`     | LibreOffice    | ❌  | ✅ Native        | ✅ Full     | Requires LibreOffice (optional)       |
| **Plain Text**    | `.txt`     | Rust           | ❌  | ❌               | ✅ Basic    | Streaming parser for multi-GB files   |
| **Markdown**      | `.md`      | Rust           | ❌  | ❌               | ✅ Enhanced | Extracts headers, links, code blocks  |
| **Rich Text**     | `.rtf`     | Pandoc         | ❌  | ❌               | ✅ Basic    | Rich Text Format                      |
| **EPUB**          | `.epub`    | Pandoc         | ❌  | ❌               | ✅ Full     | E-book format                         |
| **ODT**           | `.odt`     | Pandoc         | ❌  | ✅ Native        | ✅ Full     | OpenDocument Text                     |

### Image Formats

| Format        | Extensions                     | Implementation  | OCR | Notes                    |
| ------------- | ------------------------------ | --------------- | --- | ------------------------ |
| **JPEG**      | `.jpg`, `.jpeg`                | Rust (image-rs) | ✅  | Most common image format |
| **PNG**       | `.png`                         | Rust (image-rs) | ✅  | Lossless compression     |
| **TIFF**      | `.tiff`, `.tif`                | Rust (image-rs) | ✅  | Multi-page support       |
| **BMP**       | `.bmp`                         | Rust (image-rs) | ✅  | Windows bitmap           |
| **GIF**       | `.gif`                         | Rust (image-rs) | ✅  | Animated support         |
| **WEBP**      | `.webp`                        | Rust (image-rs) | ✅  | Modern web format        |
| **JPEG 2000** | `.jp2`, `.jpx`, `.jpm`, `.mj2` | Rust (image-rs) | ✅  | Advanced JPEG            |

### Spreadsheet Formats

| Format             | Extensions | Implementation  | Table Extraction | Metadata | Notes                    |
| ------------------ | ---------- | --------------- | ---------------- | -------- | ------------------------ |
| **Excel (Modern)** | `.xlsx`    | Rust (calamine) | ✅ Markdown      | ✅ Full  | Fastest Excel extraction |
| **Excel (Legacy)** | `.xls`     | Rust (calamine) | ✅ Markdown      | ✅ Full  | Binary format (BIFF)     |
| **Excel (Macro)**  | `.xlsm`    | Rust (calamine) | ✅ Markdown      | ✅ Full  | Macro-enabled workbooks  |
| **Excel (Binary)** | `.xlsb`    | Rust (calamine) | ✅ Markdown      | ✅ Full  | Binary Office Open XML   |
| **OpenDocument**   | `.ods`     | Rust (calamine) | ✅ Markdown      | ✅ Full  | OpenDocument Spreadsheet |
| **CSV**            | `.csv`     | Pandoc          | ✅ Markdown      | ❌       | Comma-separated values   |
| **TSV**            | `.tsv`     | Pandoc          | ✅ Markdown      | ❌       | Tab-separated values     |

### Presentation Formats

| Format                  | Extensions | Implementation     | Image Extraction | Table Extraction | Metadata | Notes                           |
| ----------------------- | ---------- | ------------------ | ---------------- | ---------------- | -------- | ------------------------------- |
| **PowerPoint (Modern)** | `.pptx`    | Rust (python-pptx) | ✅               | ✅ Markdown      | ✅ Full  | Office Open XML                 |
| **PowerPoint (Legacy)** | `.ppt`     | LibreOffice        | ✅               | ✅ Markdown      | ✅ Full  | Requires LibreOffice (optional) |

### Web & Structured Formats

| Format   | Extensions      | Implementation       | Features                            | Notes                    |
| -------- | --------------- | -------------------- | ----------------------------------- | ------------------------ |
| **HTML** | `.html`, `.htm` | Python (markdownify) | Image extraction, link preservation | Web pages                |
| **XML**  | `.xml`          | Rust (quick-xml)     | Streaming parser, element tracking  | Multi-GB file support    |
| **SVG**  | `.svg`          | Rust (quick-xml)     | XML extraction                      | Scalable vector graphics |
| **JSON** | `.json`         | Python (stdlib)      | Intelligent text field detection    | Structured data          |
| **YAML** | `.yaml`, `.yml` | Python (pyyaml)      | Nested structure preservation       | Configuration files      |
| **TOML** | `.toml`         | Python (stdlib)      | Structure preservation              | Configuration files      |

### Email Formats

| Format  | Extensions | Implementation     | Attachment Extraction | Metadata | Notes                  |
| ------- | ---------- | ------------------ | --------------------- | -------- | ---------------------- |
| **EML** | `.eml`     | Rust (mail-parser) | ✅                    | ✅ Full  | RFC 822 email messages |
| **MSG** | `.msg`     | Rust (mail-parser) | ✅                    | ✅ Full  | Outlook email messages |

### Academic & Technical Formats

| Format               | Extensions | Implementation | Features                | Notes                |
| -------------------- | ---------- | -------------- | ----------------------- | -------------------- |
| **LaTeX**            | `.tex`     | Pandoc         | Math formula extraction | TeX documents        |
| **BibTeX**           | `.bib`     | Pandoc         | Bibliography parsing    | Citation databases   |
| **Jupyter**          | `.ipynb`   | Pandoc         | Code cell extraction    | Jupyter notebooks    |
| **reStructuredText** | `.rst`     | Pandoc         | Directive parsing       | Python documentation |
| **Org Mode**         | `.org`     | Pandoc         | Outline structure       | Emacs Org files      |

**Total Supported Formats**: 50+ file types across 8 categories

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
