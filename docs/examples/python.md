# Python Examples

This page provides comprehensive examples of using Kreuzberg with Python. All example code is available in the [`examples/python/`](https://github.com/Goldziher/kreuzberg/tree/main/examples/python) directory.

## Installation

```bash
pip install kreuzberg

# With optional features
pip install "kreuzberg[ocr,api,cli]"

# With all features
pip install "kreuzberg[all]"
```

## Basic Extraction

The `basic.py` example demonstrates fundamental extraction patterns including synchronous and asynchronous extraction, working with bytes, accessing metadata, and batch processing.

### Simple Extraction

```python
--8<-- "examples/python/basic.py:12:17"
```

### Extraction with Configuration

```python
--8<-- "examples/python/basic.py:19:27"
```

### Async Extraction

```python
--8<-- "examples/python/basic.py:29:32"
```

### Extract from Bytes

```python
--8<-- "examples/python/basic.py:34:38"
```

### Accessing Metadata

```python
--8<-- "examples/python/basic.py:46:57"
```

### Batch Processing

```python
--8<-- "examples/python/batch.py:10:26"
```

### Parallel Batch Processing

For better performance with multiple files, use async batch processing:

```python
--8<-- "examples/python/batch.py:28:36"
```

### Error Handling

```python
--8<-- "examples/python/batch.py:38:57"
```

## OCR Extraction

The `ocr.py` example shows how to extract text from scanned documents and images using Tesseract OCR.

### Basic OCR

```python
--8<-- "examples/python/ocr.py:10:20"
```

### OCR with Custom Language

```python
--8<-- "examples/python/ocr.py:22:32"
```

### Force OCR on Text PDFs

Sometimes you want to extract images and run OCR even if the PDF already has text:

```python
--8<-- "examples/python/ocr.py:34:46"
```

### OCR from Images

```python
--8<-- "examples/python/ocr.py:48:58"
```

### Table Extraction with OCR

```python
--8<-- "examples/python/ocr.py:60:76"
```

### Custom PSM Mode

Page Segmentation Mode (PSM) controls how Tesseract analyzes the page layout:

```python
--8<-- "examples/python/ocr.py:78:90"
```

## Custom OCR Backends

The `custom_ocr.py` example demonstrates how to implement custom OCR backends for services like Google Cloud Vision, Azure Computer Vision, or custom ML models.

### Implementing a Custom OCR Backend

```python
--8<-- "examples/python/custom_ocr.py:9:70"
```

### Registering and Using Custom OCR

```python
--8<-- "examples/python/custom_ocr.py:130:139"
```

### Azure Computer Vision Example

```python
--8<-- "examples/python/custom_ocr.py:74:128"
```

## Custom PostProcessors

The `custom_postprocessor.py` example shows how to create PostProcessor plugins for custom data transformation, enrichment, and validation.

### PII Redaction PostProcessor

```python
--8<-- "examples/python/custom_postprocessor.py:9:45"
```

### Metadata Enrichment PostProcessor

```python
--8<-- "examples/python/custom_postprocessor.py:48:76"
```

### Text Normalization PostProcessor

```python
--8<-- "examples/python/custom_postprocessor.py:79:108"
```

### Keyword Extraction PostProcessor

```python
--8<-- "examples/python/custom_postprocessor.py:111:148"
```

### External API PostProcessor

```python
--8<-- "examples/python/custom_postprocessor.py:151:180"
```

### Registering and Using PostProcessors

```python
--8<-- "examples/python/custom_postprocessor.py:183:206"
```

## Configuration Options

### ExtractionConfig

The `ExtractionConfig` dataclass controls extraction behavior:

```python
from kreuzberg import ExtractionConfig, OcrConfig, ChunkingConfig

config = ExtractionConfig(
    # Quality processing
    enable_quality_processing=True,

    # Caching
    use_cache=True,

    # OCR configuration
    ocr=OcrConfig(
        backend="tesseract",
        language="eng",
    ),

    # Force OCR even for text-based PDFs
    force_ocr=False,

    # Chunking for large documents
    chunking=ChunkingConfig(
        max_chars=1000,
        max_overlap=100,
    ),
)
```

### OcrConfig

Configure OCR behavior:

```python
from kreuzberg import OcrConfig, TesseractConfig

ocr_config = OcrConfig(
    backend="tesseract",  # "tesseract", "easyocr", "paddleocr"
    language="eng",       # Language code
    tesseract_config=TesseractConfig(
        psm=6,                      # Page segmentation mode
        oem=3,                      # OCR Engine Mode
        enable_table_detection=True,
        dpi=300,
    ),
)
```

### ChunkingConfig

Configure content chunking for large documents:

```python
from kreuzberg import ChunkingConfig

chunking_config = ChunkingConfig(
    max_chars=1000,   # Maximum characters per chunk
    max_overlap=100,  # Overlap between chunks
)
```

## Working with Results

### ExtractionResult

The `ExtractionResult` dataclass contains all extraction information:

```python
result = extract_file("document.pdf")

# Extracted text content
print(result.content)

# MIME type
print(result.mime_type)

# Metadata (varies by document type)
if result.metadata.pdf:
    print(f"Pages: {result.metadata.pdf.page_count}")
    print(f"Author: {result.metadata.pdf.author}")
    print(f"Title: {result.metadata.pdf.title}")

# Extracted tables
for table in result.tables:
    print(table.markdown)

# Detected languages (if language detection enabled)
if result.detected_languages:
    print(f"Languages: {result.detected_languages}")

# Chunks (if chunking enabled)
if result.chunks:
    for i, chunk in enumerate(result.chunks):
        print(f"Chunk {i + 1}: {len(chunk)} chars")
```

## Error Handling

All errors inherit from `KreuzbergError`:

```python
from kreuzberg import (
    KreuzbergError,
    ValidationError,
    ParsingError,
    OCRError,
    MissingDependencyError,
)

try:
    result = extract_file("document.pdf")
except ValidationError as e:
    print(f"Validation failed: {e}")
except ParsingError as e:
    print(f"Parsing failed: {e}")
except OCRError as e:
    print(f"OCR failed: {e}")
except MissingDependencyError as e:
    print(f"Missing dependency: {e}")
except KreuzbergError as e:
    print(f"Extraction failed: {e}")
```

## Advanced Topics

### Plugin Management

```python
from kreuzberg import (
    register_post_processor,
    unregister_post_processor,
    clear_post_processors,
    register_ocr_backend,
)

# Register custom plugin
register_post_processor(MyPostProcessor())

# Unregister by name
unregister_post_processor("my_processor")

# Clear all plugins
clear_post_processors()

# Register custom OCR backend
register_ocr_backend(MyOCRBackend())
```

### Performance Tips

1. **Use batch processing** for multiple files
2. **Enable caching** for repeated extractions
3. **Use async APIs** for I/O-bound workloads
4. **Configure OCR DPI** appropriately (300 DPI is usually sufficient)
5. **Use quality processing** only when needed (adds overhead)

### Language Detection

```python
from kreuzberg import ExtractionConfig, LanguageDetectionConfig

config = ExtractionConfig(
    language_detection=LanguageDetectionConfig(
        min_confidence=0.7,  # Minimum confidence threshold
    ),
)

result = extract_file("document.pdf", config=config)
if result.detected_languages:
    print(f"Detected languages: {result.detected_languages}")
```

## Next Steps

- **[TypeScript Examples](typescript.md)** - Examples for Node.js/TypeScript
- **[Rust Examples](rust.md)** - Examples for Rust applications
- **[Plugin Development](../plugins/python-postprocessor.md)** - Deep dive into Python plugins
- **[API Reference](../api/python.md)** - Complete Python API documentation
