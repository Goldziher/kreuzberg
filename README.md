# Kreuzberg

[![Discord](https://img.shields.io/badge/Discord-Join%20our%20community-7289da)](https://discord.gg/pXxagNK2zN)
[![PyPI version](https://badge.fury.io/py/kreuzberg.svg)](https://badge.fury.io/py/kreuzberg)
[![Documentation](https://img.shields.io/badge/docs-GitHub_Pages-blue)](https://goldziher.github.io/kreuzberg/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Kreuzberg is a Python library for text extraction from documents. It provides a unified interface for extracting text from PDFs, images, office documents, and more, with both async and sync APIs.

## Why Kreuzberg?

- **Simple and Hassle-Free**: Clean API that just works, without complex configuration
- **Local Processing**: No external API calls or cloud dependencies required
- **Resource Efficient**: Lightweight processing without GPU requirements
- **Format Support**: Comprehensive support for documents, images, and text formats
- **Multiple OCR Engines**: Support for Tesseract, EasyOCR, and PaddleOCR
- **Metadata Extraction**: Get document metadata alongside text content
- **Table Extraction**: Extract tables from documents using the excellent GMFT library
- **Modern Python**: Built with async/await, type hints, and a functional-first approach
- **Permissive OSS**: MIT licensed with permissively licensed dependencies

## Quick Start

```bash
pip install kreuzberg
```

Install pandoc:

```bash
# Ubuntu/Debian
sudo apt-get install tesseract-ocr pandoc

# macOS
brew install tesseract pandoc

# Windows
choco install -y tesseract pandoc
```

The tesseract OCR engine is the default OCR engine. You can decide not to use it - and then either use one of the two alternative OCR engines, or have no OCR at all.

### Alternative OCR engines

```bash
# Install with EasyOCR support
pip install "kreuzberg[easyocr]"

# Install with PaddleOCR support
pip install "kreuzberg[paddleocr]"
```

## Quick Example

```python
import asyncio
from kreuzberg import extract_file

async def main():
    # Extract text from a PDF
    result = await extract_file("document.pdf")
    print(result.content)

    # Extract text from an image
    result = await extract_file("scan.jpg")
    print(result.content)

    # Extract text from a Word document
    result = await extract_file("report.docx")
    print(result.content)

asyncio.run(main())
```

## Documentation

For comprehensive documentation, visit our [GitHub Pages](https://goldziher.github.io/kreuzberg/):

- [Getting Started](https://goldziher.github.io/kreuzberg/getting-started/) - Installation and basic usage
- [User Guide](https://goldziher.github.io/kreuzberg/user-guide/) - In-depth usage information
- [API Reference](https://goldziher.github.io/kreuzberg/api-reference/) - Detailed API documentation
- [Examples](https://goldziher.github.io/kreuzberg/examples/) - Code examples for common use cases
- [OCR Configuration](https://goldziher.github.io/kreuzberg/user-guide/ocr-configuration/) - Configure OCR engines
- [OCR Backends](https://goldziher.github.io/kreuzberg/user-guide/ocr-backends/) - Choose the right OCR engine

## Supported Formats

Kreuzberg supports a wide range of document formats:

- **Documents**: PDF, DOCX, RTF, TXT, EPUB, etc.
- **Images**: JPG, PNG, TIFF, BMP, GIF, etc.
- **Spreadsheets**: XLSX, XLS, CSV, etc.
- **Presentations**: PPTX, PPT, etc.
- **Web Content**: HTML, XML, etc.

## OCR Engines

Kreuzberg supports multiple OCR engines:

- **Tesseract** (Default): Lightweight, fast startup, requires system installation
- **EasyOCR**: Good for many languages, pure Python, but downloads models on first use
- **PaddleOCR**: Excellent for Asian languages, pure Python, but downloads models on first use

For comparison and selection guidance, see the [OCR Backends](https://goldziher.github.io/kreuzberg/user-guide/ocr-backends/) documentation.

## Contributing

We welcome contributions! Please see our [Contributing Guide](docs/contributing.md) for details on setting up your development environment and submitting pull requests.

## License

This library is released under the MIT license.
