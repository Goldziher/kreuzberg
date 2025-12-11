# Migrating from v3 to v4

Kreuzberg v4 represents a complete architectural rewrite with a Rust-first design. This guide helps you migrate from v3 to v4.

## Overview of Changes

v4 introduces several major changes:

- **Rust Core**: Complete rewrite of core extraction logic in Rust for significant performance improvements
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

```bash title="Terminal"
# Install v3 (deprecated)
pip install kreuzberg==3.x

# Install v4 (current)
pip install kreuzberg>=4.0

# Install with all optional features
pip install "kreuzberg[all]"
```

### TypeScript (New in v4)

```bash title="Terminal"
npm install @kreuzberg/node
```

### Rust (New in v4)

```toml title="Cargo.toml"
[dependencies]
kreuzberg = "4.0"
```

## API Changes

### Python API

#### Import Changes

```python title="Python"
# v3 imports
from kreuzberg import extract_file, ExtractionConfig

# v4 imports (same public API, internal structure changed)
from kreuzberg import extract_file, ExtractionConfig
```

#### Configuration Changes

```python title="Python"
# v3 configuration (flat structure)
from kreuzberg import ExtractionConfig

config = ExtractionConfig(
    enable_ocr=True,
    ocr_language="eng",
    use_quality_processing=True,
)

# v4 configuration (nested dataclasses)
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

```python title="Python"
# v3 batch extraction
from kreuzberg import batch_extract

results = batch_extract(["file1.pdf", "file2.pdf"])

# v4 batch extraction (renamed function)
from kreuzberg import batch_extract_files

results = batch_extract_files(["file1.pdf", "file2.pdf"])
```

#### Error Handling

```python title="Python"
# v3 error handling (single exception type)
from kreuzberg import KreuzbergException

try:
    result = extract_file("doc.pdf")
except KreuzbergException as e:
    print(f"Error: {e}")

# v4 error handling (typed exception hierarchy)
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

```python title="Python"
# v3 OCR configuration (flat parameters)
config = ExtractionConfig(
    enable_ocr=True,
    ocr_language="eng",
    ocr_psm=6,
)

# v4 OCR configuration (structured backend configuration)
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

```python title="Python"
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
    use_cache=True,
    enable_quality_processing=True,
    ocr=OcrConfig(
        backend="tesseract",
        language="eng",
        tesseract_config=TesseractConfig(
            psm=6,
            oem=3,
        ),
    ),
    force_ocr=False,
    chunking=ChunkingConfig(
        max_chars=1000,
        max_overlap=100,
    ),
    images=ImageExtractionConfig(
        extract_images=True,
        target_dpi=300,
        max_image_dimension=4096,
        auto_adjust_dpi=True,
        min_dpi=72,
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
        min_confidence=0.7,
        detect_multiple=True,
    ),
    postprocessor=PostProcessorConfig(
        enabled=True,
    ),
)
```

#### Metadata Access

```python title="Python"
# v3 metadata access (dictionary-based)
result = extract_file("doc.pdf")
if "pdf" in result.metadata:
    pages = result.metadata["pdf"]["page_count"]

# v4 metadata access (typed attributes)
result = extract_file("doc.pdf")
if result.metadata.pdf:
    pages = result.metadata.pdf.page_count
```

### TypeScript API (New in v4)

TypeScript support is brand new in v4:

```typescript title="TypeScript"
import {
    extractFile,
    extractFileSync,
    ExtractionConfig,
    OcrConfig,
} from '@kreuzberg/node';

const result = await extractFile('document.pdf');

const result2 = extractFileSync('document.pdf');

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

```rust title="Rust"
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

```python title="Python"
from kreuzberg import register_document_extractor

class CustomExtractor:
    def name(self) -> str:
        return "custom"

    def supported_mime_types(self) -> list[str]:
        return ["application/x-custom"]

    def extract(self, data: bytes, mime_type: str, config) -> ExtractionResult:
        return ExtractionResult(content="extracted text", mime_type=mime_type)

register_document_extractor(CustomExtractor())
```

#### TypeScript

```typescript title="TypeScript"
import { registerPostProcessor, PostProcessorProtocol } from '@kreuzberg/node';

class CustomProcessor implements PostProcessorProtocol {
    name(): string {
        return 'custom';
    }

    process(result: ExtractionResult): ExtractionResult {
        return result;
    }
}

registerPostProcessor(new CustomProcessor());
```

### OCR Backends

```python title="Python"
# v3 OCR (Tesseract only)
config = ExtractionConfig(enable_ocr=True)

# v4 Tesseract backend
from kreuzberg import OcrConfig

config = ExtractionConfig(
    ocr=OcrConfig(backend="tesseract", language="eng")
)

# v4 EasyOCR backend (requires kreuzberg[easyocr])
config = ExtractionConfig(
    ocr=OcrConfig(backend="easyocr", language="en")
)

# v4 PaddleOCR backend (requires kreuzberg[paddleocr])
config = ExtractionConfig(
    ocr=OcrConfig(backend="paddleocr", language="en")
)

# v4 custom OCR backend
from kreuzberg import register_ocr_backend

class MyOCR:
    def name(self) -> str:
        return "my_ocr"

    def extract_text(self, image: bytes, language: str) -> str:
        return "extracted text from custom OCR"

register_ocr_backend(MyOCR())
```

### Language Detection

```python title="Python"
# v3 language detection (not available)

# v4 automatic language detection
from kreuzberg import ExtractionConfig, LanguageDetectionConfig

config = ExtractionConfig(
    language_detection=LanguageDetectionConfig(
        min_confidence=0.7,
    ),
)

result = extract_file("document.pdf", config=config)
print(result.detected_languages)
```

### Chunking

```python title="Python"
# v3 manual chunking
result = extract_file("doc.pdf")
chunks = [result.content[i:i+1000] for i in range(0, len(result.content), 1000)]

# v4 built-in chunking with overlap support
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

```python title="Python"
# v3 password-protected PDFs (not supported)

# v4 password support (requires kreuzberg[crypto])
from kreuzberg import PdfConfig

config = ExtractionConfig(
    pdf_options=PdfConfig(
        passwords=["password1", "password2"],
        extract_metadata=True,
    ),
)

result = extract_file("encrypted.pdf", config=config)
```

### Token Reduction

```python title="Python"
# v3 token reduction (not available)

# v4 token reduction for LLM processing
from kreuzberg import TokenReductionConfig

config = ExtractionConfig(
    token_reduction=TokenReductionConfig(
        mode="aggressive",
        preserve_important_words=True,
    ),
)

result = extract_file("document.pdf", config=config)
```

### Extract from Bytes

```python title="Python"
# v3 bytes extraction (limited support)

# v4 comprehensive bytes extraction API
from kreuzberg import extract_bytes, extract_bytes_sync

with open("document.pdf", "rb") as f:
    data = f.read()

result = extract_bytes_sync(data, "application/pdf")

import asyncio
result = asyncio.run(extract_bytes(data, "application/pdf"))

result = extract_bytes_sync(data, None)
```

### Table Extraction

```python title="Python"
# v3 table extraction (limited support, mixed into content)
result = extract_file("doc.pdf")

# v4 structured table extraction
result = extract_file("doc.pdf")
for table in result.tables:
    print(table.markdown)
    print(table.cells)
```

## Performance Improvements

v4 delivers significant performance improvements over v3 through its Rust-first architecture:

**Key Performance Enhancements:**

- **Rust core implementation** – Native compilation with LLVM optimizations
- **Streaming parsers** – Constant memory usage for large files (GB+)
- **Zero-copy operations** – Efficient memory management with ownership model
- **SIMD text processing** – Parallel operations for hot paths
- **Async concurrency** – True parallelism without GIL limitations
- **Smart caching** – Content-based deduplication

See the [Performance Guide](../concepts/performance.md) for detailed explanations of optimization techniques and architecture benefits.

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

```python title="Python"
# v4 automatic config discovery
result = extract_file("doc.pdf")

# v4 manual config loading
from kreuzberg import load_config

config = load_config("custom-config.toml")
result = extract_file("doc.pdf", config=config)
```

### Image Extraction

```python title="Python"
# v3 basic image extraction

# v4 advanced image extraction with DPI control
from kreuzberg import ImageExtractionConfig

config = ExtractionConfig(
    images=ImageExtractionConfig(
        extract_images=True,
        target_dpi=300,
        max_image_dimension=4096,
        auto_adjust_dpi=True,
        min_dpi=72,
    ),
)

result = extract_file("document.pdf", config=config)
```

### API Server

```bash title="Terminal"
# v3 API server (not available)

# v4 install REST API server
pip install "kreuzberg[api]"
python -m kreuzberg serve --host 0.0.0.0 --port 8000

# v4 CLI binary server
kreuzberg serve --port 8000

# v4 Docker server
docker run -p 8000:8000 goldziher/kreuzberg:latest
```

### MCP Server

```bash title="Terminal"
# v3 MCP server (not available)

# v4 Model Context Protocol server
python -m kreuzberg mcp

# v4 CLI binary MCP server
kreuzberg mcp
```

## Breaking Changes

### Configuration Structure

v3 used flat configuration. v4 uses nested dataclasses:

```python title="Python"
# v3 flat configuration
config = ExtractionConfig(
    enable_ocr=True,
    ocr_language="eng",
    ocr_psm=6,
    use_cache=True,
)

# v4 nested dataclasses
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

```python title="Python"
# v3 dictionary-based metadata
pages = result.metadata["pdf"]["page_count"]

# v4 typed dataclass metadata
pages = result.metadata.pdf.page_count
```

### Error Hierarchy

```python title="Python"
# v3 exception hierarchy
KreuzbergException (base)

# v4 exception hierarchy
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

#### GMFT (Give Me Formatted Tables)
v3's vision-based table extraction using TATR models. Replaced with Tesseract OCR table detection:

```python title="Python"
# v4 Tesseract table detection
config = ExtractionConfig(
    ocr=OcrConfig(
        tesseract_config=TesseractConfig(enable_table_detection=True)
    )
)
result = extract_file("doc.pdf", config=config)
```

#### Entity Extraction, Keyword Extraction, Document Classification
Removed. Use external libraries (spaCy, KeyBERT, etc.) with postprocessors if needed.

#### Other
- **ExtractorRegistry**: Custom extractors must be Rust plugins
- **HTMLToMarkdownConfig**, **JSONExtractionConfig**: Now use defaults
- **ImageOCRConfig**: Replaced by `ImageExtractionConfig`

## Migration Examples

### Basic Extraction

```python title="Python"
# v3 basic extraction
from kreuzberg import extract_file

result = extract_file("document.pdf")
print(result["content"])
print(result["metadata"])

# v4 basic extraction
from kreuzberg import extract_file

result = extract_file("document.pdf")
print(result.content)
print(result.metadata)
```

### OCR Extraction

```python title="Python"
# v3 OCR extraction
from kreuzberg import extract_file, ExtractionConfig

config = ExtractionConfig(
    enable_ocr=True,
    ocr_language="eng",
)

result = extract_file("scanned.pdf", config=config)

# v4 OCR extraction
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

```python title="Python"
# v3 batch processing
from kreuzberg import batch_extract

results = batch_extract(["doc1.pdf", "doc2.pdf", "doc3.pdf"])
for result in results:
    print(result["content"])

# v4 batch processing
from kreuzberg import batch_extract_files

results = batch_extract_files(["doc1.pdf", "doc2.pdf", "doc3.pdf"])
for result in results:
    print(result.content)
```

### Error Handling

```python title="Python"
# v3 error handling
from kreuzberg import extract_file, KreuzbergException

try:
    result = extract_file("doc.pdf")
except KreuzbergException as e:
    print(f"Error: {e}")

# v4 error handling
from kreuzberg import extract_file, KreuzbergError, ParsingError

try:
    result = extract_file("doc.pdf")
except ParsingError as e:
    print(f"Parsing error: {e}")
except KreuzbergError as e:
    print(f"Error: {e}")
```

## Testing Your Migration

### Automated Testing

```python title="Python"
import pytest
from kreuzberg import extract_file, ExtractionConfig

def test_basic_extraction():
    result = extract_file("tests/fixtures/sample.pdf")
    assert result.content
    assert result.mime_type == "application/pdf"

def test_ocr_extraction():
    from kreuzberg import OcrConfig

    config = ExtractionConfig(
        ocr=OcrConfig(backend="tesseract", language="eng"),
    )

    result = extract_file("tests/fixtures/scanned.pdf", config=config)
    assert result.content
    assert result.metadata.ocr

def test_batch_processing():
    from kreuzberg import batch_extract_files

    files = ["tests/fixtures/doc1.pdf", "tests/fixtures/doc2.pdf"]
    results = batch_extract_files(files)

    assert len(results) == 2
    for result in results:
        assert result.content

def test_error_handling():
    from kreuzberg import ParsingError

    with pytest.raises(ParsingError):
        extract_file("tests/fixtures/corrupted.pdf")
```

### Performance Testing

```python title="Python"
import time
from kreuzberg import extract_file, batch_extract_files

start = time.time()
result = extract_file("large_document.pdf")
print(f"Single file: {time.time() - start:.2f}s")

files = [f"document{i}.pdf" for i in range(100)]
start = time.time()
results = batch_extract_files(files)
print(f"Batch (100 files): {time.time() - start:.2f}s")
```

## Getting Help

- **Documentation**: [https://docs.kreuzberg.dev](https://docs.kreuzberg.dev)
- **Examples**: See [Python API Reference](../reference/api-python.md), [TypeScript API Reference](../reference/api-typescript.md), [Rust API Reference](../reference/api-rust.md)
- **Issues**: [GitHub Issues](https://github.com/kreuzberg-dev/kreuzberg/issues)
- **Changelog**: [CHANGELOG.md](../CHANGELOG.md)

## Deprecation Timeline

- **v3.x**: Maintenance mode (bug fixes only)
- **v4.0**: Current stable release
- **v3 EOL**: June 2025 (no further updates)

We recommend migrating to v4 as soon as possible to benefit from performance improvements and new features.
