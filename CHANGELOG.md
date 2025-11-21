# Changelog

All notable changes to Kreuzberg will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [4.0.0-rc.1] - Unreleased

### Major Release - Complete Architecture Rewrite

Kreuzberg v4 represents a complete architectural rewrite, transforming from a Python-only library into a multi-language document intelligence framework with a high-performance Rust core.

### Architecture Changes

#### Rust-First Design
- **Complete Rust Core Rewrite** (`crates/kreuzberg`): All extraction logic now implemented in Rust for maximum performance
- **Standalone Rust Crate**: Can be used directly in Rust projects without Python dependencies
- **10-50x Performance Improvements**: Text processing, streaming parsers, and I/O operations significantly faster
- **Memory Efficiency**: Streaming parsers for multi-GB XML/text files with constant memory usage
- **Type Safety**: Strong typing throughout the extraction pipeline

#### Multi-Language Support
- **Python**: PyO3 bindings (`crates/kreuzberg-py`) with native Python extensions
- **TypeScript/Node.js**: NAPI-RS bindings (`crates/kreuzberg-node`) for native Node modules
- **Ruby**: Magnus bindings (`packages/ruby/ext/kreuzberg_rb/native`) with native Ruby extensions
- **Java**: Panama/FFM bindings (`packages/java`, `crates/kreuzberg-ffi`) delivering native access for JVM applications
- **Rust**: Direct usage of `kreuzberg` crate in Rust applications
- **CLI**: Rust-based CLI (`crates/kreuzberg-cli`) with improved performance

### New Features

#### Plugin System
- **PostProcessor Plugins**: Transform extraction results (Python, TypeScript, Rust)
- **Validator Plugins**: Enforce quality requirements with fail-fast validation (Python, TypeScript, Rust)
- **Custom OCR Backends**: Integrate cloud OCR or custom ML models (Python, TypeScript, Rust)
  - **NEW: TypeScript/JavaScript OCR Backend Support**: Complete NAPI-RS ThreadsafeFunction bridge for JavaScript OCR backends
  - **Guten OCR Backend**: First-class TypeScript OCR implementation using @gutenye/ocr-node (PaddleOCR + ONNX Runtime)
  - **JSON Serialization Bridge**: Efficient data transfer between TypeScript and Rust across FFI boundaries
- **Custom Document Extractors**: Add support for new file formats (Rust)
- **Cross-Language Plugin Architecture**: Plugins can call between languages via FFI

#### Language Detection
- **Automatic Language Detection**: Fast language detection using `fast-langdetect`
- **Multi-Language Support**: Detect multiple languages in a single document
- **Configurable Confidence Thresholds**: Control detection sensitivity
- **Available in**: `ExtractionResult.detected_languages`

#### RAG & Embeddings Support
- **Automatic Embedding Generation**: Generate embeddings for text chunks using ONNX models via fastembed-rs
- **RAG-Optimized Presets**: 4 pre-configured presets (fast, balanced, quality, multilingual)
  - `fast`: 384-dim AllMiniLML6V2Q (~22M params) - Quick prototyping
  - `balanced`: 768-dim BGEBaseENV15 (~109M params) - Production default
  - `quality`: 1024-dim BGELargeENV15 (~335M params) - Maximum accuracy
  - `multilingual`: 768-dim MultilingualE5Base (100+ languages)
- **Model Caching**: Thread-safe model cache with automatic download management
- **Batch Processing**: Efficient batch embedding generation with configurable batch size
- **Embedding Normalization**: Optional L2 normalization for similarity search
- **Custom Model Paths**: Configure custom cache directories for model storage
- **Chunk Integration**: Embeddings automatically generated and attached to chunks via `Chunk.embedding`
- **Available in**: All languages (Rust, Python, TypeScript)

#### Image Extraction
- **Native Image Extraction**: Extract embedded images from PDFs and PowerPoint presentations
- **Rich Metadata**: Format, dimensions, colorspace, bits per component, page number
- **Cross-Language Raw Bytes**: Returns raw image bytes (not PIL objects) for maximum compatibility
- **Nested OCR Support**: Each extracted image can have an optional nested `ocr_result` field
- **Clean API Design**: Images stored in `ExtractionResult.images` list with all metadata inline
- **No Backward Compatibility Required**: New v4-only feature with clean, forward-looking design
- **Supported Formats**: PDF (via `lopdf`), PowerPoint (via Python `python-pptx`)

#### Enhanced Extraction

**XML Extraction**:
- Streaming XML parser using `quick-xml`
- Memory-efficient processing of multi-GB XML files
- Element counting and unique element tracking
- Preserves text content while filtering XML structure

**Plain Text & Markdown**:
- Streaming line-by-line parser for multi-GB text files
- Markdown metadata extraction: headers, links, code blocks
- Word count, line count, character count tracking
- CRLF line ending support

**PowerPoint (PPTX) Extraction**:
- Custom XML parser using `roxmltree` for Office Open XML format
- Position-based text sorting (Y-primary, X-secondary) for accurate reading order
- Table detection and extraction
- List formatting (bulleted and numbered lists)
- Image extraction with optional OCR integration
- Text formatting preservation (bold, italic, underline)
- Hyperlink detection and extraction
- Speaker notes extraction
- Comprehensive slide processing (30+ test cases covering complex scenarios)

**Stopwords System**:
- **64 Language Support**: Comprehensive stopword collections for Afrikaans, Arabic, Bulgarian, Bengali, Breton, Catalan, Czech, Danish, German, Greek, English, Esperanto, Spanish, Estonian, Basque, Persian, Finnish, French, Irish, Galician, Gujarati, Hausa, Hebrew, Hindi, Croatian, Hungarian, Armenian, Indonesian, Italian, Japanese, Kannada, Korean, Kurdish, Latin, Lithuanian, Latvian, Malayalam, Marathi, Malay, Nepali, Dutch, Norwegian, Polish, Portuguese, Romanian, Russian, Sinhala, Slovak, Slovenian, Somali, Sesotho, Swedish, Swahili, Tamil, Telugu, Thai, Tagalog, Turkish, Ukrainian, Urdu, Vietnamese, Yoruba, Chinese, Zulu
- **Compile-Time Embedding**: All stopword lists embedded in Rust binary using `include_str!()` macro
- **Zero Runtime I/O**: No file system access required, eliminating deployment dependencies
- **Automatic Integration**: Used by keyword extraction (YAKE/RAKE) and token reduction features

**Comprehensive Metadata Extraction**:

v4 introduces native metadata extraction across all major document formats:

**PDF** (native Rust extraction via `lopdf`):
- Title, subject, authors, keywords
- Created/modified dates, creator, producer
- Page count, page dimensions, PDF version
- Encryption status
- Auto-generated document summary

**Office Documents** (native Office Open XML parsing):
- **DOCX**: Core properties (Dublin Core metadata), app properties (page/word/character/line/paragraph counts, template, editing time), custom properties
- **XLSX**: Core properties, app properties (worksheet names, sheet count), custom properties
- **PPTX**: Core properties, app properties (slide count, notes, hidden slides, slide titles), custom properties
- Automatic merging with Pandoc metadata for DOCX (Pandoc takes precedence for conflicts)
- Non-blocking extraction (falls back gracefully if metadata unavailable)

**Email** (via `mail-parser`):
- From, to, cc, bcc addresses
- Message ID, subject, date
- Attachment filenames

**Images** (via `image` crate + `kamadak-exif`):
- Width, height, format
- Comprehensive EXIF data (camera settings, GPS, timestamps, etc.)

**XML** (via Rust streaming parser):
- Element count
- Unique element names

**Plain Text / Markdown** (via Rust streaming parser):
- Line count, word count, character count
- **Markdown only**: Headers, links, code blocks

**Structured Data** (JSON/YAML/TOML):
- Field count
- Format type

**HTML** (via `html-to-markdown-rs`):
- Comprehensive structured metadata extraction enabled by default
- Parses YAML frontmatter and populates `HtmlMetadata` struct:
  - Standard meta tags: title, description, keywords, author
  - Open Graph: og:title, og:description, og:image, og:url, og:type, og:site_name
  - Twitter Card: twitter:card, twitter:title, twitter:description, twitter:image, twitter:site, twitter:creator
  - Navigation: base_href, canonical URL
  - Link relations: link_author, link_license, link_alternate
- YAML frontmatter automatically stripped from markdown content
- Accessible via `ExtractionResult.metadata.html`

**Pandoc-Only Formats** (metadata via Pandoc subprocess):
- ODT, EPUB, LaTeX, reStructuredText, RTF, Typst, Jupyter Notebooks, FictionBook, Org Mode, DocBook, JATS, OPML
- Extracts whatever metadata Pandoc provides (varies by format)

**Key Improvements from v3**:
- PDF: Pure Rust `lopdf` instead of Python `playa-pdf` for better performance
- Office: Comprehensive native metadata extraction merged with Pandoc (v3 relied solely on Pandoc)
- All metadata extraction is non-blocking and gracefully handles failures
- **Python Type Safety**: All metadata types now have proper `TypedDict` definitions with comprehensive field typing
  - `PdfMetadata`, `ExcelMetadata`, `EmailMetadata`, `PptxMetadata`, `ArchiveMetadata`
  - `ImageMetadata`, `XmlMetadata`, `TextMetadata`, `HtmlMetadata`
  - `OcrMetadata`, `ImagePreprocessingMetadata`, `ErrorMetadata`
  - IDE autocomplete and type checking for all metadata fields

**Legacy MS Office Support**:
- LibreOffice conversion for `.doc` and `.ppt` files
- Automatic fallback to modern format extractors
- Optional system dependency (graceful degradation)

**PDF Improvements**:
- Better text extraction with pdfium-render
- Improved image extraction
- Force OCR mode for text-based PDFs
- Password-protected PDF support (with `crypto` extra)

**OCR Enhancements**:
- Table detection and reconstruction
- Configurable Tesseract PSM modes
- Custom OCR backend support
- Image preprocessing and DPI adjustment
- OCR result caching

### API Changes

#### Core Extraction Functions

**Async-First Design**:
```python
# Async (primary API)
result = await extract_file("document.pdf")
result = await extract_bytes(data, "application/pdf")
results = await batch_extract_files(["doc1.pdf", "doc2.pdf"])

# Sync variants available
result = extract_file_sync("document.pdf")
result = extract_bytes_sync(data, "application/pdf")
results = batch_extract_files_sync(["doc1.pdf", "doc2.pdf"])
```

**New TypeScript/Node.js API**:
```typescript
import { extractFile, extractFileSync, ExtractionConfig } from '@goldziher/kreuzberg';

// Async
const result = await extractFile('document.pdf');

// Sync
const result = extractFileSync('document.pdf');

// With configuration
const config = new ExtractionConfig({ enableQualityProcessing: true });
const result = await extractFile('document.pdf', null, config);
```

**Rust API**:
```rust
use kreuzberg::{extract_file, ExtractionConfig};

#[tokio::main]
async fn main() -> kreuzberg::Result<()> {
    let config = ExtractionConfig::default();
    let result = extract_file("document.pdf", None, &config).await?;
    println!("Extracted: {}", result.content);
    Ok(())
}
```

#### Configuration

**Strongly-Typed Configuration**:
- All configuration uses typed structs/classes (no more dictionaries)
- `ExtractionConfig`, `OcrConfig`, `ChunkingConfig`, etc.
- Compile-time validation of configuration options
- Better IDE autocomplete and type checking

**Configuration File Support**:
- TOML, YAML, and JSON configuration files
- Automatic discovery from current/parent directories
- `kreuzberg.toml`, `kreuzberg.yaml`, or `kreuzberg.json`
- CLI, API server, and MCP server all support config files

#### Result Types

**Enhanced ExtractionResult**:
```python
@dataclass
class ExtractionResult:
    content: str
    mime_type: str
    metadata: Metadata  # Strongly-typed metadata
    tables: List[ExtractedTable]
    detected_languages: Optional[List[str]]  # NEW in v4
    chunks: Optional[List[str]]
```

**Strongly-Typed Metadata**:
- `PdfMetadata`, `ExcelMetadata`, `EmailMetadata`, `ImageMetadata`, etc.
- Type-safe access to format-specific metadata
- No more dictionary casting or key errors

### Plugin System

#### PostProcessors
```python
from kreuzberg import register_post_processor, ExtractionResult

class MyPostProcessor:
    def name(self) -> str:
        return "my_processor"

    def process(self, result: ExtractionResult) -> ExtractionResult:
        # Transform result
        return result

register_post_processor(MyPostProcessor())
```

#### Validators
```python
from kreuzberg import register_validator, ExtractionResult

class MyValidator:
    def name(self) -> str:
        return "my_validator"

    def validate(self, result: ExtractionResult) -> None:
        if len(result.content) < 10:
            raise ValidationError("Content too short")

register_validator(MyValidator())
```

#### Custom OCR Backends
```python
from kreuzberg import register_ocr_backend

class CloudOCR:
    def name(self) -> str:
        return "cloud_ocr"

    def extract_text(self, image_bytes: bytes, language: str) -> str:
        # Call cloud OCR API
        return extracted_text

register_ocr_backend(CloudOCR())
```

### Performance

- **10-50x faster** text processing operations (streaming parsers)
- **Memory-efficient** streaming for multi-GB files
- **Parallel batch processing** with configurable concurrency
- **SIMD optimizations** for text processing hot paths
- **Zero-copy operations** where possible

### Docker Images

All Docker images include LibreOffice, Pandoc, and Tesseract by default:

- `goldziher/kreuzberg:4.0.0-rc.1` - Core image with Tesseract OCR
- `goldziher/kreuzberg:4.0.0-rc.1-easyocr` - Core + EasyOCR
- `goldziher/kreuzberg:4.0.0-rc.1-paddle` - Core + PaddleOCR
- `goldziher/kreuzberg:4.0.0-rc.1-vision-tables` - Core + vision-based table extraction
- `goldziher/kreuzberg:4.0.0-rc.1-all` - All features included

### Installation

**Python**:
```bash
pip install kreuzberg               # Core functionality
pip install "kreuzberg[api]"        # With API server
pip install "kreuzberg[easyocr]"    # With EasyOCR
pip install "kreuzberg[all]"        # All features
```

**TypeScript/Node.js**:
```bash
npm install @goldziher/kreuzberg
# or
pnpm add @goldziher/kreuzberg
```

**Rust**:
```toml
[dependencies]
kreuzberg = "4.0"
```

**CLI** (Homebrew):
```bash
brew install goldziher/tap/kreuzberg
```

**CLI** (Cargo):
```bash
cargo install kreuzberg-cli
```

### Breaking Changes from v3

#### Architecture
- **Rust core required**: Python package now includes Rust binaries (PyO3 bindings)
- **Binary wheels only**: No more pure-Python installation
- **Minimum versions**: Python 3.10+, Node.js 18+, Rust 1.75+

#### API Changes
- **Async-first API**: Primary API is now async, sync variants have `_sync` suffix
- **Configuration**: All config uses typed classes, not dictionaries
- **Metadata**: Strongly-typed metadata replaces free-form dictionaries
- **Function renames**: `extract()` → `extract_file()`, `extract_bytes()` is new
- **Batch API**: `batch_extract()` → `batch_extract_files()` with async support

#### Removed Features
- **Pure-Python API**: No longer available (use v3 for pure Python)
- **Old configuration format**: Dictionary-based config no longer supported
- **Legacy extractors**: Some Python-only extractors migrated to Rust
- **GMFT (Give Me Formatted Tables)**: Vision-based table extraction using TATR (Table Transformer) models removed
  - v3's GMFT used deep learning models for sophisticated table detection and parsing
  - Provided polars DataFrames, PIL Images, and multi-level header support
  - v4 replaces this with **native Tesseract-based table detection** (OCR-based, faster, simpler)
  - Configure via `TesseractConfig.enable_table_detection=True`
  - Returns `ExtractedTable` objects with cells (2D list) and markdown output
  - For advanced vision-based table extraction, use v3.x or specialized libraries
- **Entity Extraction (spaCy)**: Named entity recognition removed - use external NER libraries with postprocessors
- **Keyword Extraction (KeyBERT)**: Automatic keyword extraction removed - use external keyword extractors with postprocessors
- **Document Classification**: Automatic document type detection removed - use external classifiers with postprocessors

#### Migration Path
See [Migration Guide](https://docs.kreuzberg.dev/migration/v3-to-v4/) for detailed migration instructions.

### Documentation

- **New Documentation Site**: https://docs.kreuzberg.dev
- **Multi-Language Examples**: Python, TypeScript, and Rust examples
- **Plugin Development Guides**: Comprehensive guides for each language
- **API Reference**: Auto-generated from docstrings
- **Architecture Documentation**: Detailed system architecture explanations

### Testing

- **95%+ Test Coverage**: Comprehensive test suite in Python, TypeScript, and Rust
- **Integration Tests**: Real-world document testing
- **Benchmark Suite**: Performance comparison with other extraction libraries
- **CI/CD**: Automated testing on Linux, macOS, and Windows

### Bug Fixes

- Fixed memory leaks in PDF extraction
- Improved error handling and error messages
- Better Unicode support in text extraction
- Fixed table extraction edge cases
- Resolved deadlocks in plugin system

### Security

- All dependencies audited and updated
- No known security vulnerabilities
- Sandboxed subprocess execution (Pandoc, LibreOffice)
- Input validation on all user-provided data

### Contributors

Kreuzberg v4 was a major undertaking. Thank you to all contributors!

---

## [3.22.0] - 2025-11-21

### Fixed
- Restored public export for `OCRConfig` in v3 compatibility layer to unblock consumers relying on the legacy type.

### Changed
- Updated v3 dependencies to current versions.

---

## Migration Resources

- **Documentation**: https://docs.kreuzberg.dev
- **Migration Guide**: https://docs.kreuzberg.dev/migration/v3-to-v4/
- **Examples**: https://github.com/Goldziher/kreuzberg/tree/v4-dev/examples
- **Support**: https://github.com/Goldziher/kreuzberg/issues

[4.0.0-rc.1]: https://github.com/Goldziher/kreuzberg/compare/v4.0.0-rc.0...v4.0.0-rc.1
[4.0.0-rc.0]: https://github.com/Goldziher/kreuzberg/compare/v3.21.0...v4.0.0-rc.0
[3.22.0]: https://github.com/Goldziher/kreuzberg/compare/v3.21.0...v3.22.0
