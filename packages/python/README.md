# Kreuzberg for Python

[![PyPI](https://img.shields.io/pypi/v/kreuzberg)](https://pypi.org/project/kreuzberg/)
[![Crates.io](https://img.shields.io/crates/v/kreuzberg)](https://crates.io/crates/kreuzberg)
[![npm](https://img.shields.io/npm/v/@goldziher/kreuzberg)](https://www.npmjs.com/package/@goldziher/kreuzberg)
[![RubyGems](https://img.shields.io/gem/v/kreuzberg)](https://rubygems.org/gems/kreuzberg)
[![Python Versions](https://img.shields.io/pypi/pyversions/kreuzberg)](https://pypi.org/project/kreuzberg/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Documentation](https://img.shields.io/badge/docs-kreuzberg.dev-blue)](https://kreuzberg.dev)

High-performance document intelligence for Python. Extract text, metadata, and structured information from PDFs, Office documents, images, and 50+ formats.

**Powered by a Rust core** – Native performance for document extraction.

> **🚀 Version 4.0.0 Release Candidate**
> This is a pre-release version. We invite you to test the library and [report any issues](https://github.com/Goldziher/kreuzberg/issues) you encounter.

## Installation

```bash
pip install kreuzberg
```

### With OCR Support

```bash
pip install "kreuzberg[easyocr]"
pip install "kreuzberg[paddleocr]"
```

### All Features

```bash
pip install "kreuzberg[all]"
```

## Quick Start

### Simple Extraction

```python
from kreuzberg import extract_file_sync

result = extract_file_sync("document.pdf")
print(result.content)
```

### Async Extraction (Recommended)

```python
from kreuzberg import extract_file

result = await extract_file("document.pdf")
print(result.content)
```

### Batch Processing

```python
from kreuzberg import batch_extract_files

files = ["doc1.pdf", "doc2.docx", "doc3.xlsx"]
results = await batch_extract_files(files)

for result in results:
    print(result.content)
```

## OCR Support

### Tesseract (Default)

```python
from kreuzberg import extract_file_sync, ExtractionConfig, OcrConfig

config = ExtractionConfig(
    ocr=OcrConfig(backend="tesseract", language="eng")
)

result = extract_file_sync("scanned.pdf", config=config)
```

### EasyOCR (GPU-Accelerated)

```python
from kreuzberg import extract_file_sync, ExtractionConfig, OcrConfig

config = ExtractionConfig(
    ocr=OcrConfig(backend="easyocr", language="en")
)

result = extract_file_sync(
    "photo.jpg",
    config=config,
    easyocr_kwargs={"use_gpu": True}
)
```

### PaddleOCR (Complex Layouts)

```python
from kreuzberg import extract_file_sync, ExtractionConfig, OcrConfig

config = ExtractionConfig(
    ocr=OcrConfig(backend="paddleocr", language="ch")
)

result = extract_file_sync(
    "invoice.pdf",
    config=config,
    paddleocr_kwargs={"use_angle_cls": True}
)
```

## Table Extraction

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

result = extract_file_sync("invoice.pdf", config=config)

for table in result.tables:
    print(table.markdown)
    print(table.cells)
```

## Configuration

### Complete Configuration Example

```python
from kreuzberg import (
    extract_file_sync,
    ExtractionConfig,
    OcrConfig,
    TesseractConfig,
    ChunkingConfig,
    ImageExtractionConfig,
    PdfConfig,
    TokenReductionConfig,
    LanguageDetectionConfig,
)

config = ExtractionConfig(
    use_cache=True,
    enable_quality_processing=True,
    ocr=OcrConfig(
        backend="tesseract",
        language="eng",
        tesseract_config=TesseractConfig(
            psm=6,
            enable_table_detection=True,
            min_confidence=50.0,
        ),
    ),
    force_ocr=False,
    chunking=ChunkingConfig(
        max_chars=1000,
        max_overlap=200,
    ),
    images=ImageExtractionConfig(
        extract_images=True,
        target_dpi=300,
        max_image_dimension=4096,
        auto_adjust_dpi=True,
    ),
    pdf_options=PdfConfig(
        extract_images=True,
        passwords=["password1", "password2"],
        extract_metadata=True,
    ),
    token_reduction=TokenReductionConfig(
        mode="moderate",
        preserve_important_words=True,
    ),
    language_detection=LanguageDetectionConfig(
        enabled=True,
        min_confidence=0.8,
        detect_multiple=False,
    ),
)

result = extract_file_sync("document.pdf", config=config)
```

### HTML Conversion Options & Batch Concurrency

```python
from kreuzberg import ExtractionConfig

config = ExtractionConfig(
    max_concurrent_extractions=8,
    html_options={
        "extract_metadata": True,
        "wrap": True,
        "wrap_width": 100,
        "strip_tags": ["script", "style"],
        "preprocessing": {"enabled": True, "preset": "standard"},
    },
)
```

## Metadata Extraction

```python
from kreuzberg import extract_file_sync

result = extract_file_sync("document.pdf")

if result.images:
    print(f"Extracted {len(result.images)} inline images")

if result.chunks:
    print(f"First chunk tokens: {result.chunks[0]['metadata']['token_count']}")

print(result.metadata.get("pdf", {}))
print(result.metadata.get("language"))
print(result.metadata.get("format"))

if "pdf" in result.metadata:
    pdf_meta = result.metadata["pdf"]
    print(f"Title: {pdf_meta.get('title')}")
    print(f"Author: {pdf_meta.get('author')}")
    print(f"Pages: {pdf_meta.get('page_count')}")
    print(f"Created: {pdf_meta.get('creation_date')}")
```

## Password-Protected PDFs

```python
from kreuzberg import extract_file_sync, ExtractionConfig, PdfConfig

config = ExtractionConfig(
    pdf_options=PdfConfig(
        passwords=["password1", "password2", "password3"]
    )
)

result = extract_file_sync("protected.pdf", config=config)
```

## Language Detection

```python
from kreuzberg import extract_file_sync, ExtractionConfig, LanguageDetectionConfig

config = ExtractionConfig(
    language_detection=LanguageDetectionConfig(enabled=True)
)

result = extract_file_sync("multilingual.pdf", config=config)
print(result.detected_languages)
```

## Text Chunking

```python
from kreuzberg import extract_file_sync, ExtractionConfig, ChunkingConfig

config = ExtractionConfig(
    chunking=ChunkingConfig(
        max_chars=1000,
        max_overlap=200,
    )
)

result = extract_file_sync("long_document.pdf", config=config)

for chunk in result.chunks:
    print(chunk)
```

## Extract from Bytes

```python
from kreuzberg import extract_bytes_sync

with open("document.pdf", "rb") as f:
    data = f.read()

result = extract_bytes_sync(data, "application/pdf")
print(result.content)
```

## API Reference

### Extraction Functions

- `extract_file(file_path, mime_type=None, config=None, **kwargs)` – Async extraction
- `extract_file_sync(file_path, mime_type=None, config=None, **kwargs)` – Sync extraction
- `extract_bytes(data, mime_type, config=None, **kwargs)` – Async extraction from bytes
- `extract_bytes_sync(data, mime_type, config=None, **kwargs)` – Sync extraction from bytes
- `batch_extract_files(paths, config=None, **kwargs)` – Async batch extraction
- `batch_extract_files_sync(paths, config=None, **kwargs)` – Sync batch extraction
- `batch_extract_bytes(data_list, mime_types, config=None, **kwargs)` – Async batch from bytes
- `batch_extract_bytes_sync(data_list, mime_types, config=None, **kwargs)` – Sync batch from bytes

### Configuration Classes

- `ExtractionConfig` – Main configuration
- `OcrConfig` – OCR settings
- `TesseractConfig` – Tesseract-specific options
- `ChunkingConfig` – Text chunking settings
- `ImageExtractionConfig` – Image extraction settings
- `PdfConfig` – PDF-specific options
- `TokenReductionConfig` – Token reduction settings
- `LanguageDetectionConfig` – Language detection settings

### Result Types

- `ExtractionResult` – Main result object with `content`, `metadata`, `tables`, `detected_languages`, `chunks`
- `ExtractedTable` – Table with `cells`, `markdown`, `page_number`
- `Metadata` – Typed metadata dictionary

### Exceptions

- `KreuzbergError` – Base exception
- `ValidationError` – Invalid configuration or input
- `ParsingError` – Document parsing failure
- `OCRError` – OCR processing failure
- `MissingDependencyError` – Missing optional dependency

## Examples

### Custom Processing

```python
from kreuzberg import extract_file_sync

result = extract_file_sync("document.pdf")

text = result.content
text = text.lower()
text = text.replace("old", "new")

print(text)
```

### Multiple Files with Progress

```python
from kreuzberg import extract_file_sync
from pathlib import Path

files = list(Path("documents").glob("*.pdf"))
results = []

for i, file in enumerate(files, 1):
    print(f"Processing {i}/{len(files)}: {file.name}")
    result = extract_file_sync(str(file))
    results.append((file.name, result))

for name, result in results:
    print(f"{name}: {len(result.content)} characters")
```

### Filter by Language

```python
from kreuzberg import extract_file_sync, ExtractionConfig, LanguageDetectionConfig

config = ExtractionConfig(
    language_detection=LanguageDetectionConfig(enabled=True)
)

result = extract_file_sync("document.pdf", config=config)

if result.detected_languages and "en" in result.detected_languages:
    print("English document detected")
    print(result.content)
```

## System Dependencies

### Tesseract OCR (Required for OCR)

```bash
brew install tesseract
```

```bash
sudo apt-get install tesseract-ocr
```

### LibreOffice (Optional, for .doc and .ppt)

```bash
brew install libreoffice
```

```bash
sudo apt-get install libreoffice
```

### Pandoc (Optional, for some formats)

```bash
brew install pandoc
```

```bash
sudo apt-get install pandoc
```

## Troubleshooting

### Import Error: No module named '_kreuzberg'

This usually means the Rust extension wasn't built correctly. Try:

```bash
pip install --force-reinstall --no-cache-dir kreuzberg
```

### OCR Not Working

Make sure Tesseract is installed:

```bash
tesseract --version
```

### Memory Issues with Large PDFs

Use streaming or enable chunking:

```python
config = ExtractionConfig(
    chunking=ChunkingConfig(max_chars=1000)
)
```

## Complete Documentation

**[https://kreuzberg.dev](https://kreuzberg.dev)**

- [Installation Guide](https://kreuzberg.dev/getting-started/installation/)
- [User Guide](https://kreuzberg.dev/user-guide/)
- [API Reference](https://kreuzberg.dev/api-reference/)
- [Format Support](https://kreuzberg.dev/formats/)
- [OCR Backends](https://kreuzberg.dev/user-guide/ocr-backends/)

## License

MIT License - see [LICENSE](../../LICENSE) for details.
