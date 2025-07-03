# Kreuzberg

[![Discord](https://img.shields.io/badge/Discord-Join%20our%20community-7289da)](https://discord.gg/pXxagNK2zN)
[![PyPI version](https://badge.fury.io/py/kreuzberg.svg)](https://badge.fury.io/py/kreuzberg)
[![Documentation](https://img.shields.io/badge/docs-GitHub_Pages-blue)](https://goldziher.github.io/kreuzberg/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

**High-performance Python library for text extraction from documents.** Extract text from PDFs, images, office documents, and more with both async and sync APIs.

📖 **[Complete Documentation](https://goldziher.github.io/kreuzberg/)**

## Why Kreuzberg?

- **🚀 Fastest Performance**: [Benchmarked](https://github.com/Goldziher/python-text-extraction-libs-benchmarks) as the fastest text extraction library
- **💾 Memory Efficient**: 14x smaller than alternatives (71MB vs 1GB+)
- **⚡ Dual APIs**: Only library with both sync and async support
- **🔧 Zero Configuration**: Works out of the box with sane defaults
- **🏠 Local Processing**: No cloud dependencies or external API calls
- **📦 Rich Format Support**: PDFs, images, Office docs, HTML, and more
- **🔍 Multiple OCR Engines**: Tesseract, EasyOCR, and PaddleOCR support
- **🐳 Production Ready**: CLI, REST API, and Docker images included

## Quick Start

### Installation

```bash
# Basic installation
pip install kreuzberg

# With optional features
pip install "kreuzberg[cli,api]"        # CLI + REST API
pip install "kreuzberg[easyocr,gmft]"   # EasyOCR + table extraction
pip install "kreuzberg[all]"            # Everything
```

### System Dependencies

```bash
# Ubuntu/Debian
sudo apt-get install tesseract-ocr pandoc

# macOS
brew install tesseract pandoc

# Windows
choco install tesseract pandoc
```

### Basic Usage

```python
import asyncio
from kreuzberg import extract_file

async def main():
    # Extract from any document type
    result = await extract_file("document.pdf")
    print(result.content)
    print(result.metadata)

asyncio.run(main())
```

## Deployment Options

### 🐳 Docker (Recommended)

```bash
# Run API server
docker run -p 8000:8000 goldziher/kreuzberg:3.4.0

# Extract files
curl -X POST http://localhost:8000/extract -F "data=@document.pdf"
```

Available variants: `3.4.0`, `3.4.0-easyocr`, `3.4.0-paddle`, `3.4.0-gmft`, `3.4.0-all`

### 🌐 REST API

```bash
# Install and run
pip install "kreuzberg[api]"
litestar --app kreuzberg._api.main:app run

# Health check
curl http://localhost:8000/health

# Extract files
curl -X POST http://localhost:8000/extract -F "data=@file.pdf"
```

### 💻 Command Line

```bash
# Install CLI
pip install "kreuzberg[cli]"

# Extract to stdout
kreuzberg extract document.pdf

# JSON output with metadata
kreuzberg extract document.pdf --output-format json --show-metadata

# Batch processing
kreuzberg extract *.pdf --output-dir ./extracted/
```

## Supported Formats

| Category          | Formats                        |
| ----------------- | ------------------------------ |
| **Documents**     | PDF, DOCX, DOC, RTF, TXT, EPUB |
| **Images**        | JPG, PNG, TIFF, BMP, GIF, WEBP |
| **Spreadsheets**  | XLSX, XLS, CSV, ODS            |
| **Presentations** | PPTX, PPT, ODP                 |
| **Web**           | HTML, XML, MHTML               |
| **Archives**      | Support via extraction         |

## Performance

**Fastest extraction speeds** with minimal resource usage:

| Library       | Speed          | Memory        | Size        | Success Rate |
| ------------- | -------------- | ------------- | ----------- | ------------ |
| **Kreuzberg** | ⚡ **Fastest** | 💾 **Lowest** | 📦 **71MB** | ✅ **100%**  |
| Unstructured  | 2-3x slower    | 2x higher     | 146MB       | 95%          |
| MarkItDown    | 3-4x slower    | 3x higher     | 251MB       | 90%          |
| Docling       | 4-5x slower    | 10x higher    | 1,032MB     | 85%          |

> **Rule of thumb**: Use async API for complex documents and batch processing (up to 4.5x faster)

## Documentation

### Quick Links

- [Installation Guide](https://goldziher.github.io/kreuzberg/getting-started/installation/) - Setup and dependencies
- [User Guide](https://goldziher.github.io/kreuzberg/user-guide/) - Comprehensive usage guide
- [API Reference](https://goldziher.github.io/kreuzberg/api-reference/) - Complete API documentation
- [Docker Guide](https://goldziher.github.io/kreuzberg/user-guide/docker/) - Container deployment
- [REST API](https://goldziher.github.io/kreuzberg/user-guide/api-server/) - HTTP endpoints
- [CLI Guide](https://goldziher.github.io/kreuzberg/cli/) - Command-line usage
- [OCR Configuration](https://goldziher.github.io/kreuzberg/user-guide/ocr-configuration/) - OCR engine setup

## Advanced Features

- **📊 Table Extraction**: Extract tables from PDFs with GMFT
- **🧩 Content Chunking**: Split documents for RAG applications
- **🎯 Custom Extractors**: Extend with your own document handlers
- **🔧 Configuration**: Flexible TOML-based configuration
- **🪝 Hooks**: Pre/post-processing customization
- **🌍 Multi-language OCR**: 100+ languages supported
- **⚙️ Metadata Extraction**: Rich document metadata
- **🔄 Batch Processing**: Efficient bulk document processing

## License

MIT License - see [LICENSE](LICENSE) for details.

______________________________________________________________________

<div align="center">

**[Documentation](https://goldziher.github.io/kreuzberg/) • [PyPI](https://pypi.org/project/kreuzberg/) • [Docker Hub](https://hub.docker.com/r/goldziher/kreuzberg) • [Discord](https://discord.gg/pXxagNK2zN)**

Made with ❤️ by the [Kreuzberg contributors](https://github.com/Goldziher/kreuzberg/graphs/contributors)

</div>
