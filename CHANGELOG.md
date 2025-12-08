# Changelog

All notable changes to Kreuzberg will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [4.0.0-rc.6] - 2025-12-07

### Release Candidate 6 - FFI Core Feature & CI/Build Improvements

#### New Features

**FFI Bindings**:
- Added `core` feature for kreuzberg-ffi without embeddings support
  - Provides lightweight FFI build option excluding ONNX Runtime dependency
  - Enables Windows MinGW compatibility for Go bindings
  - Includes HTML processing and all document extraction features
  - Use `--no-default-features --features core` for MinGW builds

#### Bug Fixes

**ODT Extraction**:
- Fixed ODT table extraction producing duplicate content
  - Table cells were being extracted twice: once as markdown tables (correct) and again as raw paragraphs (incorrect)
  - Root cause: XML traversal using `.descendants()` included nested table cell content as document-level text
  - Solution: Changed to only process direct children of `<office:text>` element, isolating table content
  - Impact: ODT extraction now produces clean output without cell duplication
- Enhanced ODT metadata extraction to match Office Open XML capabilities
  - Added comprehensive metadata extraction from `meta.xml` (OpenDocument standard)
  - New `OdtProperties` struct supports all OpenDocument metadata fields
  - Extracts: title, subject, creator, initial-creator, keywords, description, dates, language
  - Document statistics: page count, word count, character count, paragraph count, table count, image count
  - Metadata extraction now consistent between ODT, DOCX, XLSX, and PPTX formats
  - Impact: ODT files now provide rich metadata comparable to other Office formats

**Go Bindings**:
- Fixed Windows MinGW builds by disabling embeddings feature
  - Windows ONNX Runtime only provides MSVC .lib files incompatible with MinGW
  - Go bindings on Windows now use `core` feature (no embeddings)
  - Full features (including embeddings) remain available on Linux, macOS, and Windows MSVC
- Fixed test execution to use `test_documents` instead of `.kreuzberg` cache
  - Ensures reproducible test runs without relying on user cache directory
  - Improves CI/CD reliability and test isolation

**CI/CD Infrastructure**:
- Upgraded `upload-artifact` from v4 to v5 for compatibility with `download-artifact@v6`
  - Fixes artifact version mismatch causing benchmark and CI failures
  - Affects 10 workflow files with 42 total changes
  - Resolves "artifact not found" errors in multi-job workflows
- Fixed RUSTFLAGS handling in `setup-onnx-runtime` action
  - Now appends to existing RUSTFLAGS instead of overwriting
  - Preserves `-C target-feature=+crt-static` for Windows GNU builds
- Fixed Go Windows CI artifact download path causing linker failures
  - Changed download path from `target` to `.` to prevent double-nesting (target/target/...)
  - Linker can now find libkreuzberg_ffi.dll at correct location
  - Added debug logging to show directory structure after artifact download

**Ruby Bindings**:
- Fixed rb-sys links conflict in gem build
  - Removed rb-sys vendoring, now uses version 0.9.119 from crates.io
  - Resolves Cargo error: "package rb-sys links to native library rb, but it conflicts with previous package"
  - Allows Cargo to unify rb-sys dependency across magnus and kreuzberg-rb

**C# E2E Tests**:
- Fixed OCR tests failing with empty content
  - Added render_config_expression function to C# E2E generator
  - Tests now pass proper OCR config JSON instead of null
  - Regenerated all C# tests with tesseract backend configuration
- Fixed metadata array contains assertion for single value in array
  - Extended ValueContains method to handle value-in-array case
  - Fixes sheet_names metadata assertions in Excel tests

**Python Bindings**:
- Fixed missing format_type in text extraction metadata
  - TypstExtractor and LatexExtractor incorrectly claimed text/plain MIME type
  - Removed text/plain from both extractors' supported types
  - PlainTextExtractor now correctly handles text/plain with proper TextMetadata
  - Metadata now includes format_type, line_count, word_count, character_count
  - Added unit test for Metadata serialization to verify format field flattening

## [4.0.0-rc.5] - 2025-12-01

### Release Candidate 5 - macOS Binary Fix & Complete Pandoc Removal

#### Breaking Changes

**Complete Pandoc Removal**:
- **Removed all Pandoc dependencies** from v4 codebase (100% native Rust extractors)
  - Deleted 7 Pandoc code files (3,006 lines)
  - Removed `pandoc-fallback` feature flag from Cargo.toml
  - Removed Pandoc installation from all CI/CD workflows (Linux, macOS, Windows)
  - Removed Pandoc from Docker images (saving ~500MB-1GB per image)
  - Updated all documentation to reflect native-only approach
  - Deleted 160+ Pandoc baseline test files
- **Native Rust extractors** now handle all 12 previously Pandoc-supported formats:
  - LaTeX, EPUB, BibTeX, Typst, Jupyter, FictionBook, DocBook, JATS, OPML
  - Org-mode, reStructuredText, RTF, Markdown variants
- **Benefits**: Simpler installation (no system dependencies), faster CI builds (~2-5 min improvement), smaller Docker images, pure Rust codebase
- **Migration**: No action required - native extractors are drop-in replacements with equivalent or better quality

#### Bug Fixes

**Build System**:
- Fixed macOS CLI binary missing libpdfium.dylib (dyld error at runtime)
  - Build script now correctly copies libpdfium.dylib to target-specific directory when using --target flag
  - Resolves: `dyld: Library not loaded: @rpath/libpdfium.dylib`
  - Impact: macOS CLI binary now functional in releases

**Windows Go Builds**:
- Fixed persistent Windows Go CI failures where `ring` crate failed with MSVC toolchain detection
  - Set GNU as default Rust toolchain on Windows: `rustup default stable-x86_64-pc-windows-gnu`
  - Updated Rust build cache keys to include target architecture, preventing MSVC cache reuse
  - Added MSYS2 UCRT64 setup with comprehensive GNU toolchain configuration
  - Resolves: `TARGET = Some(x86_64-pc-windows-msvc)` error in build scripts
  - Impact: Windows Go bindings now build successfully with proper GNU toolchain isolation

**Ruby CI Bundler 4.0 Compatibility**:
- Fixed gem installation failures on macOS and Linux caused by empty environment variables
  - Removed job-level `GEM_HOME=""` and `BUNDLE_PATH=""` that broke non-Windows builds
  - These variables are now only set on Windows with proper short paths for MAX_PATH mitigation
  - Updated to `bundle update --all` (deprecated `bundle update` removed in Bundler 4.0)
  - Resolves: `ERROR: While executing gem ... (Errno::ENOENT) No such file or directory @ dir_s_mkdir`
  - Impact: Ruby gem builds now succeed on all platforms with Bundler 4.0.0

**Note**: rc.4 workflow fixes for Python, Node, Ruby, and Maven were committed after rc.4 tag, causing those packages not to publish. All fixes are now present for rc.5.

---

## [4.0.0-rc.4] - 2025-12-01

### Release Candidate 4 - Critical CI/CD and Build Fixes

#### Bug Fixes

**CI/CD Workflow Fixes**:
- Fixed RubyGems action version (v2 doesn't exist, now using v1.0.0)
- Fixed pnpm workspace configuration (replaced invalid `--cwd` flag with `-C`)
- Fixed Docker environment variables (undefined `$LD_LIBRARY_PATH` in Dockerfiles)
- Fixed Maven credentials timing (env vars now available when setup-java generates settings.xml)
- Fixed Maven GPG configuration (modernized arguments to `--pinentry-mode=loopback` format)
- Removed release notes update job from publish workflow (not needed)

**Core Library Fixes**:
- Fixed Tesseract OCR test failure (corrected API call ordering: set_image before set_source_resolution)
- Fixed Go Windows CGO linking (build Rust FFI with x86_64-pc-windows-gnu target for MinGW compatibility)

**Testing**:
- All 24 Tesseract tests now pass (was 23/24 in rc.3)
- Go bindings now build successfully on Windows

---

## [4.0.0-rc.2] - 2025-11-30

### Release Candidate 2 - C# Support & Infrastructure Improvements

#### Breaking Changes

**TypeScript/Node.js Package Restructuring**:
- NPM package renamed from `kreuzberg` to `@kreuzberg/node` (scoped package)
- Platform-specific packages now use `@kreuzberg/{platform}` naming scheme
- TypeScript source consolidated into `crates/kreuzberg-node` (merged with native bindings)
- Migration: Replace `import { ... } from 'kreuzberg'` with `import { ... } from '@kreuzberg/node'`
- See [Migration Guide](docs/migration/v4-npm-scoped-packages.md) for details

#### New Features

**C#/.NET Bindings**:
- Complete C# bindings using .NET 9+ Foreign Function & Memory API
- Native FFI bridge via `kreuzberg-ffi` C library
- Supports .NET 9+ on Linux, macOS, and Windows
- Package: `Kreuzberg` on NuGet (pending publication)
- Full feature parity with other language bindings

**Documentation Improvements**:
- New v3 documentation site with MkDocs
- Comprehensive multi-language code examples for all 7 supported languages
- API reference documentation for all bindings
- Migration guides and tutorials

#### Bug Fixes

**CI/CD Fixes**:
- Fixed all CI workflow failures (ONNX Runtime, Maven dependencies, path triggers)
- Fixed benchmark harness configuration validation (single-file mode)
- Added ONNX Runtime installation to all relevant CI workflows
- Updated Maven plugins to latest compatible versions with enforcer plugin

**Core Library Fixes**:
- Added lock poisoning recovery to embeddings model cache
- Improved Tesseract tessdata path detection for Linux systems
- Fixed Python async test configuration (pytest-asyncio)
- Resolved Rust formatting issues with edition 2024 let-chain syntax

**Benchmark Harness Improvements**:
- Added comprehensive adapter registration diagnostics
- Fixed Tesseract benchmark path resolution
- Improved Python async benchmark output with performance metrics
- Added missing `--max-concurrent` parameter validation

**Code Quality**:
- Fixed Python linting issues (ruff complexity, mypy type parameters)
- Resolved all clippy warnings
- Fixed C# generated file formatting
- Improved error handling across all bindings

#### Developer Experience

**Build System**:
- Go FFI library and Ruby native extension build fixes
- Improved build reproducibility
- Better error messages during compilation

**Testing**:
- 268+ Rust tests passing
- 13/13 Java tests passing
- 32/32 benchmark harness tests passing
- Fixed 27 Python async test failures
- Unblocked 250+ tests across all language bindings

---

## [4.0.0-rc.1] - 2025-11-23

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


**Key Improvements from v3**:
- PDF: Pure Rust `lopdf` instead of Python `playa-pdf` for better performance
- Office: Comprehensive native metadata extraction via Office Open XML parsing
- All metadata extraction is non-blocking and gracefully handles failures
- **Python Type Safety**: All metadata types now have proper `TypedDict` definitions with comprehensive field typing
  - `PdfMetadata`, `ExcelMetadata`, `EmailMetadata`, `PptxMetadata`, `ArchiveMetadata`
  - `ImageMetadata`, `XmlMetadata`, `TextMetadata`, `HtmlMetadata`
  - `OcrMetadata`, `ImagePreprocessingMetadata`, `ErrorMetadata`
  - IDE autocomplete and type checking for all metadata fields

**Legacy MS Office Support**:
- LibreOffice conversion for `.doc` and `.ppt` files
- Automatic fallback to modern format extractors after LibreOffice conversion
- Optional system dependency (graceful degradation if unavailable)

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
import { extractFile, extractFileSync, ExtractionConfig } from 'kreuzberg';

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

All Docker images include LibreOffice and Tesseract by default:

- `kreuzberg-dev/kreuzberg:4.0.0-rc.1` - Core image with Tesseract OCR
- `kreuzberg-dev/kreuzberg:4.0.0-rc.1-easyocr` - Core + EasyOCR
- `kreuzberg-dev/kreuzberg:4.0.0-rc.1-paddle` - Core + PaddleOCR
- `kreuzberg-dev/kreuzberg:4.0.0-rc.1-vision-tables` - Core + vision-based table extraction
- `kreuzberg-dev/kreuzberg:4.0.0-rc.1-all` - All features included

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
npm install @kreuzberg/node
# or
pnpm add @kreuzberg/node
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
- Sandboxed subprocess execution (LibreOffice)
- Input validation on all user-provided data

### Contributors

Kreuzberg v4 was a major undertaking. Thank you to all contributors!

---

## [3.22.0] - 2025-11-27

### Fixed
- Always attempt EasyOCR import before raising `MissingDependencyError` to improve error handling
- Hardened HTML regexes for script/style stripping to prevent edge cases

### Added
- Test coverage for EasyOCR import path edge cases

### Changed
- Updated v3 dependencies to current versions

---

## Migration Resources

- **Documentation**: https://docs.kreuzberg.dev
- **Migration Guide**: https://docs.kreuzberg.dev/migration/v3-to-v4/
- **Examples**: https://github.com/kreuzberg-dev/kreuzberg/tree/v4-dev/examples
- **Support**: https://github.com/kreuzberg-dev/kreuzberg/issues

[4.0.0-rc.2]: https://github.com/kreuzberg-dev/kreuzberg/compare/v4.0.0-rc.1...v4.0.0-rc.2
[4.0.0-rc.1]: https://github.com/kreuzberg-dev/kreuzberg/compare/v4.0.0-rc.0...v4.0.0-rc.1
[4.0.0-rc.0]: https://github.com/kreuzberg-dev/kreuzberg/compare/v3.21.0...v4.0.0-rc.0
[3.22.0]: https://github.com/kreuzberg-dev/kreuzberg/compare/v3.21.0...v3.22.0
