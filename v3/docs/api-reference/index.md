# API Reference

Complete reference documentation for the Kreuzberg document intelligence framework's public API.

## Core Components

- [Extraction Functions](extraction-functions.md) - Functions for text extraction ([Guide](../user-guide/basic-usage.md))
- [Types](types.md) - Data structures for results and configuration ([Guide](../user-guide/extraction-configuration.md))
- [OCR Configuration](ocr-configuration.md) - OCR engine settings ([Guide](../user-guide/ocr-configuration.md))
- [Extractor Registry](extractor-registry.md) - Document extractor management ([Guide](../advanced/custom-extractors.md))
- [Exceptions](exceptions.md) - Error handling ([Examples](../getting-started/quick-start.md#error-handling))

## Public API

All components documented in this section are exported directly from the `kreuzberg` package and can be imported as follows:

```python
from kreuzberg import extract_file, ExtractionConfig, TesseractConfig  # etc.
```

## API Overview

Kreuzberg's API has four main components:

1. **Extraction Functions**: Extract text from documents
1. **Configuration Objects**: Control extraction behavior
1. **Result Objects**: Contain extracted text and metadata
1. **OCR Backends**: Pluggable OCR engines

## Examples

### Async API (Recommended)

```python
from kreuzberg import extract_file, ExtractionConfig

# Basic usage
result = await extract_file("document.pdf")

# With configuration
result = await extract_file("document.pdf", config=ExtractionConfig(force_ocr=True))
```

### Sync API

```python
from kreuzberg import extract_file_sync

# Basic usage
result = extract_file_sync("document.pdf")
```
