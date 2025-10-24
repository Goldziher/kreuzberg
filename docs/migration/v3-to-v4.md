# Migrating from v3 to v4

Kreuzberg v4 represents a complete architectural rewrite with a Rust-first design. This guide helps you migrate from v3 to v4.

## Overview of Changes

v4 introduces several major changes:

- **Rust Core**: Complete rewrite of core extraction logic in Rust for 10-50x performance improvements
- **Multi-Language Support**: Native support for Python, TypeScript, and Rust
- **Plugin System**: Trait-based plugin architecture for extensibility
- **Type Safety**: Improved type definitions across all languages
- **Breaking API Changes**: Several API changes for consistency and better ergonomics

## Quick Migration Checklist

- [ ] Update dependencies to v4
- [ ] Update import statements (some modules reorganized)
- [ ] Update configuration (new dataclasses/types)
- [ ] Update error handling (exception hierarchy changed)
- [ ] Migrate custom extractors to new plugin system
- [ ] Test thoroughly (behavior may differ in edge cases)

## Installation

### Python

```bash
# v3
pip install kreuzberg==3.x

# v4
pip install kreuzberg>=4.0

# With features
pip install "kreuzberg[all]"
```

### TypeScript (New in v4)

```bash
npm install @goldziher/kreuzberg
```

### Rust (New in v4)

```toml
[dependencies]
kreuzberg = "4.0"
```

## API Changes

### Python API

#### Import Changes

```python
# v3
from kreuzberg import extract_file, ExtractionConfig

# v4 (same, but internal structure changed)
from kreuzberg import extract_file, ExtractionConfig
```

#### Configuration Changes

```python
# v3
from kreuzberg import ExtractionConfig

config = ExtractionConfig(
    enable_ocr=True,
    ocr_language="eng",
    use_quality_processing=True,
)

# v4
from kreuzberg import ExtractionConfig, OcrConfig

config = ExtractionConfig(
    ocr=OcrConfig(
        backend="tesseract",
        language="eng",
    ),
    enable_quality_processing=True,
)
```

#### Batch Processing

```python
# v3
from kreuzberg import batch_extract

results = batch_extract(["file1.pdf", "file2.pdf"])

# v4
from kreuzberg import batch_extract_files

results = batch_extract_files(["file1.pdf", "file2.pdf"])
```

#### Error Handling

```python
# v3
from kreuzberg import KreuzbergException

try:
    result = extract_file("doc.pdf")
except KreuzbergException as e:
    print(f"Error: {e}")

# v4
from kreuzberg import KreuzbergError, ParsingError, ValidationError

try:
    result = extract_file("doc.pdf")
except ParsingError as e:
    print(f"Parsing error: {e}")
except ValidationError as e:
    print(f"Validation error: {e}")
except KreuzbergError as e:
    print(f"Error: {e}")
```

#### OCR Configuration

```python
# v3
config = ExtractionConfig(
    enable_ocr=True,
    ocr_language="eng",
    ocr_psm=6,
)

# v4
from kreuzberg import OcrConfig, TesseractConfig

config = ExtractionConfig(
    ocr=OcrConfig(
        backend="tesseract",
        language="eng",
        tesseract_config=TesseractConfig(
            psm=6,
            oem=3,
        ),
    ),
)
```

#### Complete Configuration (v4)

v4 provides extensive configuration options across all features:

```python
from kreuzberg import (
    ExtractionConfig,
    OcrConfig,
    TesseractConfig,
    ChunkingConfig,
    ImageExtractionConfig,
    PdfConfig,
    TokenReductionConfig,
    LanguageDetectionConfig,
    PostProcessorConfig,
)

config = ExtractionConfig(
    # Caching
    use_cache=True,

    # Quality processing
    enable_quality_processing=True,

    # OCR configuration
    ocr=OcrConfig(
        backend="tesseract",
        language="eng",
        tesseract_config=TesseractConfig(
            psm=6,
            oem=3,
        ),
    ),
    force_ocr=False,  # Force OCR even for text-based PDFs

    # Chunking
    chunking=ChunkingConfig(
        max_chars=1000,
        max_overlap=100,
    ),

    # Image extraction
    images=ImageExtractionConfig(
        extract_images=True,
        target_dpi=300,
        max_image_dimension=4096,
        auto_adjust_dpi=True,
        min_dpi=72,
    ),

    # PDF options
    pdf_options=PdfConfig(
        extract_images=True,
        passwords=["password1", "password2"],  # Try multiple passwords
        extract_metadata=True,
    ),

    # Token reduction
    token_reduction=TokenReductionConfig(
        mode="moderate",  # "off", "light", "moderate", "aggressive"
        preserve_important_words=True,
    ),

    # Language detection
    language_detection=LanguageDetectionConfig(
        enabled=True,
        min_confidence=0.7,
        detect_multiple=True,
    ),

    # PostProcessor configuration
    postprocessor=PostProcessorConfig(
        enabled=True,
    ),
)
```

#### Metadata Access

```python
# v3
result = extract_file("doc.pdf")
if "pdf" in result.metadata:
    pages = result.metadata["pdf"]["page_count"]

# v4
result = extract_file("doc.pdf")
if result.metadata.pdf:
    pages = result.metadata.pdf.page_count
```

### TypeScript API (New in v4)

TypeScript support is brand new in v4:

```typescript
import {
    extractFile,
    extractFileSync,
    ExtractionConfig,
    OcrConfig,
} from '@goldziher/kreuzberg';

// Async extraction
const result = await extractFile('document.pdf');

// Sync extraction
const result2 = extractFileSync('document.pdf');

// With configuration
const config = new ExtractionConfig({
    ocr: new OcrConfig({
        backend: 'tesseract',
        language: 'eng',
    }),
});

const result3 = await extractFile('document.pdf', null, config);
```

### Rust API (New in v4)

The Rust core is now available as a standalone library:

```rust
use kreuzberg::{extract_file_sync, ExtractionConfig};

fn main() -> kreuzberg::Result<()> {
    let config = ExtractionConfig::default();
    let result = extract_file_sync("document.pdf", None, &config)?;
    println!("Content: {}", result.content);
    Ok(())
}
```

## Feature Changes

### Custom Extractors

v3 had limited support for custom extractors. v4 introduces a comprehensive plugin system.

#### Python

```python
# v4 - Custom extractor
from kreuzberg import register_document_extractor

class CustomExtractor:
    def name(self) -> str:
        return "custom"

    def supported_mime_types(self) -> list[str]:
        return ["application/x-custom"]

    def extract(self, data: bytes, mime_type: str, config) -> ExtractionResult:
        # Implementation
        pass

register_document_extractor(CustomExtractor())
```

#### TypeScript

```typescript
// v4 - Custom PostProcessor
import { registerPostProcessor, PostProcessorProtocol } from '@goldziher/kreuzberg';

class CustomProcessor implements PostProcessorProtocol {
    name(): string {
        return 'custom';
    }

    process(result: ExtractionResult): ExtractionResult {
        // Implementation
        return result;
    }
}

registerPostProcessor(new CustomProcessor());
```

### OCR Backends

```python
# v3 - Only Tesseract
config = ExtractionConfig(enable_ocr=True)

# v4 - Multiple backends
from kreuzberg import OcrConfig

# Tesseract
config = ExtractionConfig(
    ocr=OcrConfig(backend="tesseract", language="eng")
)

# EasyOCR (requires kreuzberg[easyocr])
config = ExtractionConfig(
    ocr=OcrConfig(backend="easyocr", language="en")
)

# PaddleOCR (requires kreuzberg[paddleocr])
config = ExtractionConfig(
    ocr=OcrConfig(backend="paddleocr", language="en")
)

# Custom OCR backend
from kreuzberg import register_ocr_backend

class MyOCR:
    def name(self) -> str:
        return "my_ocr"

    def extract_text(self, image: bytes, language: str) -> str:
        # Implementation
        pass

register_ocr_backend(MyOCR())
```

### Language Detection

```python
# v3 - Not available

# v4 - Automatic language detection
from kreuzberg import ExtractionConfig, LanguageDetectionConfig

config = ExtractionConfig(
    language_detection=LanguageDetectionConfig(
        min_confidence=0.7,
    ),
)

result = extract_file("document.pdf", config=config)
print(result.detected_languages)  # ['eng', 'deu']
```

### Chunking

```python
# v3 - Manual chunking
result = extract_file("doc.pdf")
chunks = [result.content[i:i+1000] for i in range(0, len(result.content), 1000)]

# v4 - Built-in chunking
from kreuzberg import ChunkingConfig

config = ExtractionConfig(
    chunking=ChunkingConfig(
        max_chars=1000,
        max_overlap=100,
    ),
)

result = extract_file("doc.pdf", config=config)
for chunk in result.chunks:
    print(f"Chunk: {len(chunk)} chars")
```

### Password-Protected PDFs

```python
# v3 - Not available

# v4 - Password support (requires kreuzberg[crypto])
from kreuzberg import PdfConfig

config = ExtractionConfig(
    pdf_options=PdfConfig(
        passwords=["password1", "password2"],  # Try multiple passwords in order
        extract_metadata=True,
    ),
)

result = extract_file("encrypted.pdf", config=config)
```

### Token Reduction

```python
# v3 - Not available

# v4 - Token reduction for LLM processing
from kreuzberg import TokenReductionConfig

config = ExtractionConfig(
    token_reduction=TokenReductionConfig(
        mode="aggressive",  # "off", "light", "moderate", "aggressive"
        preserve_important_words=True,
    ),
)

result = extract_file("document.pdf", config=config)
# Content is automatically reduced while preserving meaning
```

### Extract from Bytes

```python
# v3 - Limited support

# v4 - Full bytes extraction API
from kreuzberg import extract_bytes, extract_bytes_sync

# Read file into memory
with open("document.pdf", "rb") as f:
    data = f.read()

# Sync extraction
result = extract_bytes_sync(data, "application/pdf")

# Async extraction
import asyncio
result = asyncio.run(extract_bytes(data, "application/pdf"))

# With MIME type auto-detection
result = extract_bytes_sync(data, None)  # Auto-detect MIME type
```

### Table Extraction

```python
# v3 - Limited table support
result = extract_file("doc.pdf")
# Tables mixed into content

# v4 - Structured table extraction
result = extract_file("doc.pdf")
for table in result.tables:
    print(table.markdown)  # Markdown format
    print(table.cells)     # Structured data
```

## Performance Improvements

v4 is significantly faster than v3:

| Operation | v3 Time | v4 Time | Improvement |
|-----------|---------|---------|-------------|
| PDF Extraction | 2.5s | 0.15s | 16x faster |
| OCR Processing | 5.0s | 0.8s | 6x faster |
| XML Parsing | 3.0s | 0.06s | 50x faster |
| Batch (100 files) | 180s | 12s | 15x faster |

These improvements are due to:

- Rust core implementation
- Streaming parsers for large files
- Optimized memory management
- SIMD text processing
- Efficient caching

## New Features in v4

### Plugin System

Four plugin types:

1. **DocumentExtractor** - Custom file format extractors
2. **OcrBackend** - Custom OCR engines
3. **PostProcessor** - Data transformation and enrichment
4. **Validator** - Fail-fast validation

### Multi-Language Support

v4 provides native APIs for:

- **Python** - PyO3 bindings
- **TypeScript/Node.js** - NAPI-RS bindings
- **Rust** - Direct library usage

### Configuration Discovery

```python
# v4 - Automatic config discovery
# Looks for kreuzberg.toml, kreuzberg.yaml, kreuzberg.json
result = extract_file("doc.pdf")  # Uses discovered config

# Manual config
from kreuzberg import load_config

config = load_config("custom-config.toml")
result = extract_file("doc.pdf", config=config)
```

### Image Extraction

```python
# v3 - Basic image extraction

# v4 - Advanced image extraction with DPI control
from kreuzberg import ImageExtractionConfig

config = ExtractionConfig(
    images=ImageExtractionConfig(
        extract_images=True,
        target_dpi=300,              # Target DPI for extracted images
        max_image_dimension=4096,     # Max dimension in pixels
        auto_adjust_dpi=True,         # Auto-adjust DPI for memory efficiency
        min_dpi=72,                   # Minimum DPI threshold
    ),
)

result = extract_file("document.pdf", config=config)
# Images are extracted with optimized DPI settings
```

### API Server

```bash
# v3 - Not available

# v4 - Built-in REST API server
pip install "kreuzberg[api]"
python -m kreuzberg serve --host 0.0.0.0 --port 8000

# Or via CLI binary
kreuzberg serve --port 8000

# Or via Docker
docker run -p 8000:8000 goldziher/kreuzberg:latest
```

### MCP Server

```bash
# v3 - Not available

# v4 - Model Context Protocol server for Claude Desktop
python -m kreuzberg mcp

# Or via CLI binary
kreuzberg mcp

# Configure in Claude Desktop:
# macOS: ~/Library/Application Support/Claude/claude_desktop_config.json
# Add: {"mcpServers": {"kreuzberg": {"command": "/path/to/kreuzberg", "args": ["mcp"]}}}
```

## Breaking Changes

### Configuration Structure

v3 used flat configuration. v4 uses nested dataclasses:

```python
# v3
config = ExtractionConfig(
    enable_ocr=True,
    ocr_language="eng",
    ocr_psm=6,
    use_cache=True,
)

# v4
config = ExtractionConfig(
    ocr=OcrConfig(
        backend="tesseract",
        language="eng",
        tesseract_config=TesseractConfig(psm=6),
    ),
    use_cache=True,
)
```

### Metadata Structure

v3 used dictionaries. v4 uses typed dataclasses:

```python
# v3
pages = result.metadata["pdf"]["page_count"]

# v4
pages = result.metadata.pdf.page_count
```

### Error Hierarchy

```python
# v3
KreuzbergException (base)

# v4
KreuzbergError (base)
├── ValidationError
├── ParsingError
├── OCRError
├── MissingDependencyError
├── PluginError
└── ConfigurationError
```

### Function Names

| v3 | v4 |
|----|----|
| `batch_extract()` | `batch_extract_files()` |
| `extract_bytes()` | `extract_bytes()` (same) |
| `extract_file()` | `extract_file()` (same) |

### Removed Features

- **Custom MIME detection** - Now handled internally with improved detection
- **Legacy extractor API** - Use new plugin system
- **Synchronous-only mode** - v4 is async-first with sync wrappers

## Migration Examples

### Basic Extraction

```python
# v3
from kreuzberg import extract_file

result = extract_file("document.pdf")
print(result["content"])
print(result["metadata"])

# v4
from kreuzberg import extract_file

result = extract_file("document.pdf")
print(result.content)
print(result.metadata)
```

### OCR Extraction

```python
# v3
from kreuzberg import extract_file, ExtractionConfig

config = ExtractionConfig(
    enable_ocr=True,
    ocr_language="eng",
)

result = extract_file("scanned.pdf", config=config)

# v4
from kreuzberg import extract_file, ExtractionConfig, OcrConfig

config = ExtractionConfig(
    ocr=OcrConfig(
        backend="tesseract",
        language="eng",
    ),
)

result = extract_file("scanned.pdf", config=config)
```

### Batch Processing

```python
# v3
from kreuzberg import batch_extract

results = batch_extract(["doc1.pdf", "doc2.pdf", "doc3.pdf"])
for result in results:
    print(result["content"])

# v4
from kreuzberg import batch_extract_files

results = batch_extract_files(["doc1.pdf", "doc2.pdf", "doc3.pdf"])
for result in results:
    print(result.content)
```

### Error Handling

```python
# v3
from kreuzberg import extract_file, KreuzbergException

try:
    result = extract_file("doc.pdf")
except KreuzbergException as e:
    print(f"Error: {e}")

# v4
from kreuzberg import extract_file, KreuzbergError, ParsingError

try:
    result = extract_file("doc.pdf")
except ParsingError as e:
    print(f"Parsing error: {e}")
    # Handle parsing-specific error
except KreuzbergError as e:
    print(f"Error: {e}")
    # Handle other errors
```

## Testing Your Migration

### Automated Testing

```python
# test_migration.py
import pytest
from kreuzberg import extract_file, ExtractionConfig

def test_basic_extraction():
    """Test that basic extraction works"""
    result = extract_file("tests/fixtures/sample.pdf")
    assert result.content
    assert result.mime_type == "application/pdf"

def test_ocr_extraction():
    """Test OCR extraction"""
    from kreuzberg import OcrConfig

    config = ExtractionConfig(
        ocr=OcrConfig(backend="tesseract", language="eng"),
    )

    result = extract_file("tests/fixtures/scanned.pdf", config=config)
    assert result.content
    assert result.metadata.ocr

def test_batch_processing():
    """Test batch processing"""
    from kreuzberg import batch_extract_files

    files = ["tests/fixtures/doc1.pdf", "tests/fixtures/doc2.pdf"]
    results = batch_extract_files(files)

    assert len(results) == 2
    for result in results:
        assert result.content

def test_error_handling():
    """Test error handling"""
    from kreuzberg import ParsingError

    with pytest.raises(ParsingError):
        extract_file("tests/fixtures/corrupted.pdf")
```

### Performance Testing

```python
import time
from kreuzberg import extract_file, batch_extract_files

# Single file
start = time.time()
result = extract_file("large_document.pdf")
print(f"Single file: {time.time() - start:.2f}s")

# Batch processing
files = [f"document{i}.pdf" for i in range(100)]
start = time.time()
results = batch_extract_files(files)
print(f"Batch (100 files): {time.time() - start:.2f}s")
```

## Getting Help

- **Documentation**: [https://docs.kreuzberg.dev](https://docs.kreuzberg.dev)
- **Examples**: See [Python Examples](../examples/python.md), [TypeScript Examples](../examples/typescript.md), [Rust Examples](../examples/rust.md)
- **Issues**: [GitHub Issues](https://github.com/Goldziher/kreuzberg/issues)
- **Changelog**: [CHANGELOG.md](../../CHANGELOG.md)

## Deprecation Timeline

- **v3.x**: Maintenance mode (bug fixes only)
- **v4.0**: Current stable release
- **v3 EOL**: June 2025 (no further updates)

We recommend migrating to v4 as soon as possible to benefit from performance improvements and new features.
