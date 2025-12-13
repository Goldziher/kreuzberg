# Changelog

All notable changes to Kreuzberg will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [4.0.0-rc.7] - 2025-12-12

### Added
- **WebAssembly bindings** (`@kreuzberg/wasm` npm package) for browser, Cloudflare Workers, Deno, and Bun
  - Full TypeScript API with sync and async extraction methods
  - Multi-threading support via `wasm-bindgen-rayon` and SharedArrayBuffer
  - Batch processing with `batchExtractBytes()` and `batchExtractBytesSync()`
  - Plugin system for custom post-processors, validators, and OCR backends
  - MIME type detection and configuration management
  - Comprehensive unit tests (95%+ coverage on core modules)
  - Production-ready error handling with detailed error messages
- RTF extractor now builds structured tables (markdown + cells) and parses RTF `\info` metadata (authors, dates, counts), bringing parity with DOCX/ODT fixtures.
- New pandoc-generated RTF fixtures with embedded metadata for `word_sample`, `lorem_ipsum`, and `extraction_test` to validate cross-format extraction.
- **Page tracking and metadata redesign** (#226)
  - Per-page content extraction with `PageContent` type
  - Byte-accurate page boundaries with `PageBoundary` type for O(1) lookups
  - Detailed per-page metadata with `PageInfo` type (dimensions, counts, visibility)
  - Unified page structure tracking with `PageStructure` type
  - `PageConfig` for controlling page extraction behavior
  - Automatic chunk-to-page mapping with `first_page`/`last_page` in `ChunkMetadata`
  - Format-specific support:
    - PDF: Full byte-accurate tracking with O(1) performance
    - PPTX: Slide boundary tracking
    - DOCX: Best-effort page break detection
  - Page markers in combined text for LLM context awareness

### Changed
- **BREAKING**: `ChunkMetadata` field renames for byte-accurate tracking (#226)
  - `char_start` → `byte_start` (UTF-8 byte offset)
  - `char_end` → `byte_end` (UTF-8 byte offset)
  - Existing code using `char_start`/`char_end` must be updated
  - See [migration guide](docs/migration/v3-to-v4/#field-renames-character-to-byte-offsets) for details

### Fixed
- Comprehensive lint cleanup across the crate and tests (clippy warnings resolved).
- Publish workflow now tolerates apt-managed RubyGems installations by skipping unsupported `gem update --system` during gem rebuild and installs a fallback .NET SDK when the runner lacks `dotnet`.
- Docker publish now skips pushing when the target version tag already exists, avoiding redundant builds for released images.
- Docker tag existence is checked upfront before any publish work, and per-variant publish jobs are skipped early when the version is already present.
- Added preflight checks for CLI, Go, and Rust crates to skip build/publish when the release artifacts already exist.
- Maven publishing now uses Sonatype Central's `central-publishing-maven-plugin` with auto-publish/wait and Central user-token credentials, replacing the legacy OSSRH endpoint.
- Maven Central publish timeout increased from 30 minutes to 2 hours to accommodate slower validation/publishing process.
- Python wheels are now built with `manylinux: auto` parameter (was incorrectly set to `manylinux2014` which is not a valid maturin-action value), fixing PyPI upload rejection of `linux_x86_64` platform tags.
- manylinux wheel builds now detect container type (CentOS vs Debian) and set correct `OPENSSL_LIB_DIR` paths (`/usr/lib64` for CentOS, `/usr/lib/x86_64-linux-gnu` for Debian) to avoid openssl-sys build failures in maturin builds.
- Ruby Gemfile.lock now includes x86_64-linux platform for CI compatibility on Linux runners.
- Ruby gem corruption fixed by excluding .fastembed_cache (567MB of embedding models) and target directories from gemspec fallback path.
- Java Panama FFM SIGSEGV crashes on macOS ARM64 fixed by adding explicit padding fields to FFI structs (CExtractionResult and CBatchResult) to ensure struct alignment matches between Rust and Java.
- TypeScript E2E test type error fixed in smoke.spec.ts by using proper expectation object format.
- Node.js benchmarks now have tsx as workspace dev dependency and root-level typecheck script.
- C# compilation errors (CS0136, CS0128, CS0165) resolved by fixing variable shadowing in e2e/csharp/Helpers.cs.
- Python CI timeout issues resolved by marking slow office document tests with @pytest.mark.slow and skipping them in CI.
- Go CI tests enhanced with comprehensive verbose logging and platform-specific diagnostics for better debugging.

## [4.0.0-rc.6] - 2025-12-10

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
- Aligned all workflows to Java 25
  - Updated from Java 24 to 25 across all CI and publish workflows
  - Resolves "release version 25 not supported" compilation errors
  - Affects ci-validate, ci-java, publish, and benchmarks workflows

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

## [4.0.0-rc.3] - 2025-12-01

### Release Candidate 3 - Publishing & Testing Fixes

#### Bug Fixes

**Publishing Workflow**:
- Fixed crates.io publishing order (crates-io publishing now properly sequenced)
- Fixed NuGet publishing to use API key authentication
- Resolved all remaining publish workflow failures across CI/CD pipeline
- Fixed Maven Central publishing to use NEXUS_USERNAME/PASSWORD credentials

**Language Bindings**:
- Fixed C# tests cloning JsonNode values to avoid parent assignment violations
- Resolved test failures across Ruby and Java bindings
- Updated Node binding dependencies and lockfile
- Fixed import paths in Node binding tests from src/ to dist/
- Removed incorrect dependencies from Node package.json

**Core Library**:
- Prevented ONNX Runtime mutex errors during process cleanup
- Fixed embeddings model initialization to prevent deadlocks
- Prevented OCR backend clearing from affecting other tests
- Switched from ort-load-dynamic to ort-download-binaries for better compatibility

**CLI & Binaries**:
- Included libpdfium shared library in CLI binary packages for proper runtime linking

**Documentation & Theme**:
- Updated documentation theme colors to align with new design system
- Added CONTRIBUTING.md symlink to fix broken GitHub documentation links

**CI/CD Infrastructure**:
- Restructured publish workflow for independent package publishing across languages
- Fixed dependency updates for kreuzberg-tesseract to 4.0.0-rc.2
- Updated pnpm filters for consistent @kreuzberg/node package handling
- Applied rustfmt to benchmarks and tests for code consistency

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
- See [Migration Guide](migration/v3-to-v4.md) for details

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
```python title="Python"
# Asynchronous extraction (recommended for I/O-bound operations)
result = await extract_file("document.pdf")
result = await extract_bytes(data, "application/pdf")
results = await batch_extract_files(["doc1.pdf", "doc2.pdf"])

# Synchronous variants (for simple scripts or non-async contexts)
result = extract_file_sync("document.pdf")
result = extract_bytes_sync(data, "application/pdf")
results = batch_extract_files_sync(["doc1.pdf", "doc2.pdf"])
```

**New TypeScript/Node.js API**:
```typescript title="TypeScript"
import { extractFile, extractFileSync, ExtractionConfig } from 'kreuzberg';

// Asynchronous extraction
const result = await extractFile('document.pdf');

// Synchronous extraction
const result = extractFileSync('document.pdf');

// Extraction with custom configuration (quality processing enabled)
const config = new ExtractionConfig({ enableQualityProcessing: true });
const result = await extractFile('document.pdf', null, config);
```

**Rust API**:
```rust title="Rust"
use kreuzberg::{extract_file, ExtractionConfig};

#[tokio::main]
async fn main() -> kreuzberg::Result<()> {
    // Initialize configuration with default settings
    let config = ExtractionConfig::default();
    // Perform asynchronous file extraction
    let result = extract_file("document.pdf", None, &config).await?;
    // Display extracted content
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
```python title="Python"
@dataclass
class ExtractionResult:
    content: str
    mime_type: str
    metadata: Metadata  # Format-specific strongly-typed metadata
    tables: List[ExtractedTable]
    detected_languages: Optional[List[str]]  # Language detection results (v4 feature)
    chunks: Optional[List[str]]
```

**Strongly-Typed Metadata**:
- `PdfMetadata`, `ExcelMetadata`, `EmailMetadata`, `ImageMetadata`, etc.
- Type-safe access to format-specific metadata
- No more dictionary casting or key errors

### Plugin System

#### PostProcessors
```python title="Python"
from kreuzberg import register_post_processor, ExtractionResult

class MyPostProcessor:
    def name(self) -> str:
        return "my_processor"

    def process(self, result: ExtractionResult) -> ExtractionResult:
        # Apply custom transformations to extraction result
        return result

register_post_processor(MyPostProcessor())
```

#### Validators
```python title="Python"
from kreuzberg import register_validator, ExtractionResult

class MyValidator:
    def name(self) -> str:
        return "my_validator"

    def validate(self, result: ExtractionResult) -> None:
        # Enforce minimum content length requirement
        if len(result.content) < 10:
            raise ValidationError("Content too short")

register_validator(MyValidator())
```

#### Custom OCR Backends
```python title="Python"
from kreuzberg import register_ocr_backend

class CloudOCR:
    def name(self) -> str:
        return "cloud_ocr"

    def extract_text(self, image_bytes: bytes, language: str) -> str:
        # Send image to cloud OCR service and return extracted text
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
```bash title="Terminal"
pip install kreuzberg               # Core functionality
pip install "kreuzberg[api]"        # With API server
pip install "kreuzberg[easyocr]"    # With EasyOCR
pip install "kreuzberg[all]"        # All features
```

**TypeScript/Node.js**:
```bash title="Terminal"
npm install @kreuzberg/node
# or
pnpm add @kreuzberg/node
```

**Rust**:
```toml title="Cargo.toml"
[dependencies]
kreuzberg = "4.0"
```

**CLI** (Homebrew):
```bash title="Terminal"
brew install kreuzberg-dev/tap/kreuzberg
```

**CLI** (Cargo):
```bash title="Terminal"
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

## [3.21.0] - 2025-11-05

### Major Release: Kreuzberg v4 Migration

This release introduces **Kreuzberg v4**, a complete rewrite with Rust core, polyglot bindings (Python/TypeScript/Ruby/Java/Go), and enhanced architecture. v3 remains available for legacy projects.

### Added

#### Core (Rust)
- Complete Rust core library with comprehensive document extraction pipeline
- Plugin system architecture for extensible extractors, OCR backends, postprocessors, and validators
- PDF extraction and rendering with advanced table detection
- Format extraction for Office documents (DOCX, XLSX, PPTX), HTML, XML, and plain text
- OCR subsystem with pluggable backend support (Tesseract, EasyOCR, PaddleOCR)
- Image processing utilities with EXIF extraction and format conversion
- Text processing with token reduction, chunking, keyword extraction, and language detection
- Stopwords utilities for 50+ languages
- Cache system with in-memory and persistent storage
- Embeddings support via FastEmbed
- MCP (Model Context Protocol) server integration
- CLI binary for command-line document extraction

#### Language Bindings
- **Python**: PyO3 FFI bindings with Python-idiomatic API and async support
- **TypeScript/Node.js**: NAPI-RS bindings with full type definitions
- **Ruby**: Magnus FFI bindings with RBS type definitions
- **Java**: Java 25 Foreign Function & Memory API (FFM/Panama) bindings
- **Go**: CGO bindings with Go 1.25+ support
- **C#**: FFI bindings (baseline implementation)

#### API Server & MCP
- REST API server for document extraction
- MCP server for Claude integration
- Comprehensive configuration system with builder patterns

#### Testing & Infrastructure
- 95%+ test coverage across all components
- End-to-end test suites for all language bindings (auto-generated from fixtures)
- Comprehensive Rust test suite with unit/integration/doc tests
- Multi-language CI/CD with GitHub Actions
- Docker builds for containerized deployment
- Benchmark harness for performance analysis

#### Documentation
- New documentation site: https://docs.kreuzberg.dev
- Language-specific guides for all 6+ bindings
- Plugin development documentation
- API reference with code examples
- Architecture documentation
- Migration guide from v3 to v4

### Changed

- Architecture restructured around Rust core with thin language-specific wrappers
- Build system upgraded to Rust Edition 2024 with Cargo workspace
- Dependency management via Cargo, npm/pnpm, PyPI, Maven, and Go modules
- Task automation via Taskfile.yaml
- Enhanced error handling with typed exceptions across all languages
- Configuration system redesigned for consistency across languages

### Removed

- Old v3 codebase superseded by v4 (v3 remains available in separate branch)
- Legacy Python implementation details replaced by PyO3 bindings
- Previous Node.js implementation replaced by NAPI-RS

### Security

- All dependencies audited for vulnerabilities
- Sandboxed subprocess execution (LibreOffice, Tesseract)
- Input validation on all user-provided data
- Memory safety via Rust

### Performance

- Streaming PDF extraction for memory efficiency
- Zero-copy patterns throughout Rust core
- SIMD optimizations where applicable
- ONNX Runtime for embeddings
- Async-first design for I/O operations

---

## [3.20.2] - 2025-10-11

### Fixed

- Surface missing optional dependency errors in GMFT extractor
- Stabilize aggregation data loading in benchmarks
- Make Docker E2E API checks resilient to transient failures

### Changed

- Make comparative benchmarks manual-only to reduce resource consumption

### Dependencies

- Updated dependencies to latest versions

---

## [3.20.1] - 2025-10-11

### Fixed

- Correct publish docker workflow include path

### Changed

- Optimize sdist size by excluding unnecessary files

---

## [3.20.0] - 2025-10-11

### Added

- Python 3.14 support

### Changed

- Migrate HTML extractor to html-to-markdown v2 for improved compatibility and performance

### Fixed

- Comparative benchmarks: fix frozen dataclass mutation and aggregation workflow
- CI improvements and coverage adjustments
- Add pytest-benchmark to dev dependencies for benchmark workflow

### Dependencies

- Bump astral-sh/setup-uv from 6 to 7
- Bump peter-evans/dockerhub-description from 4 to 5
- Bump actions/download-artifact from 4 to 5
- Bump actions/setup-python from 5 to 6
- Bump actions/checkout from 4 to 5

### Documentation

- Document Python 3.14 support limitations

---

## [3.19.1] - 2025-09-30

### Fixed

- Replace mocked Tesseract test with real file-based test
- Remove Rust build step from kreuzberg benchmarks workflow
- Resolve prek and mypy issues in comparative-benchmarks
- Add ruff config to comparative-benchmarks to allow benchmark patterns
- Ensure Windows Tesseract 5.5.0 HOCR output compatibility
- Properly type TypedDict configs with type narrowing and cast

### Changed

- Optimize extractor configurations for fair comparison in benchmarks
- Complete comparative-benchmarks workspace with visualization and docs generation

### Dependencies

- Updated dependencies to latest versions

### Testing

- Add regression test for Issue #149 Windows Tesseract HOCR compatibility

---

## [3.19.0] - 2025-09-29

### Added

- Enforce critical system error policy with context-aware exception handling
- Implement systematic exception handling audit across all modules
- Add German language pack support for Windows CI

### Fixed

- Align sync/async OCR pipelines and fix Tesseract PSM enum handling
- Remove magic library dependency completely
- Prevent magic import crashes in Windows tests
- Properly mock magic import in CLI tests
- Correct asyncio.gather result assertion in profiler test
- Resolve benchmark test failures and naming cleanup
- Add Windows-safe fallbacks for CLI progress and magic
- Make OCR test less brittle for character recognition
- Eliminate all Rich Console instantiations at import time
- Prevent Windows access violation in benchmarks CLI
- Update OCR test assertion to match actual error message format
- Handle ValidationError gracefully in batch processing contexts
- Correct test fixture file paths and remove hardcoded paths
- Ensure coverage job only runs when all tests pass

### Changed

- Remove verbose AI-pattern naming from components
- Enable coverage job for branches and update changelog

### Dependencies

- Add missing needs declaration to python-tests job in workflows

### Documentation

- Align documentation with v4-dev improvements
- Improve error handling policy documentation

### Performance

- Parallelize comparative benchmarks with 6-hour timeout

---

## [3.18.0] - 2025-09-27

### Added

- Make API server configuration configurable via environment variables
- Improve spaCy model auto-download with uv fallback
- Add regression tests for German image PDF extraction (issue #149)

### Changed

- Use uv to install prek for latest version
- Replace pre-commit commands with Prek in Taskfile.yml
- Update pre-commit instructions to use Prek
- Update html-to-markdown to latest version
- Auto-download missing spaCy models for entity extraction
- Fix HOCR parsing issues

### Fixed

- Resolve mypy type checking issues
- Prevent DeepSource upload when tests fail
- Make DeepSource upload optional when DEEPSOURCE_DSN is missing
- Prevent coverage-pr from running when test-pr fails

### Dependencies

- Remove pre-commit dependency from dev requirements
- Updated dependencies to latest versions

### CI/CD

- Use prek instead of pre-commit in validation workflow

### Documentation

- Update contributing guide to use prek instead of pre-commit
- Update uv.lock to reflect dependency changes and remove upload-time attributes

---

## [3.17.0] - 2025-09-17

### Added

- Add token reduction for text optimization
- Add concurrency settings to cancel in-progress workflow runs
- Optimize token reduction and update dependencies
- Optimize token reduction performance and add streaming support

### Fixed

- Resolve excessive markdown escaping in OCR output (fixes #133)
- Remove invalid table extraction tests for non-existent functions
- Ensure comprehensive test coverage in CI
- Resolve Windows path compatibility and improve test coverage
- Critical issues from second review of token reduction

### Changed

- Complete token reduction implementation overhaul

### Testing

- Improve coverage and add pragma no cover annotations

---

## [3.16.0] - 2025-09-16

### Added

- Enhance JSON extraction with schema analysis and custom field detection
- Add internal streaming optimization for html-to-markdown conversions
- Comprehensive test coverage improvements

### Fixed

- Export HTMLToMarkdownConfig in public API
- Resolve EasyOCR module-level variable issues and adjust coverage requirement
- Resolve mypy type errors and test failures
- Add xfail markers to chunking, ML-dependent, cache, and language detection tests
- Add xfail markers for EasyOCR device validation tests
- Resolve CI test failures with targeted fixes
- Add missing Any import for type annotations
- Update docker e2e workflow to use correct test filename
- Prevent docker_e2e.py from being discovered by pytest
- Resolve Windows-specific path issues in tests

### Changed

- Remove coverage fail_under threshold completely
- Remove unnecessary xfail markers

### Dependencies

- Bump actions/download-artifact from 4 to 5

### Testing

- Add comprehensive tests for API configuration caching
- Increase test coverage to meet CI requirements

---

## [3.15.0] - 2025-09-14

### Added

- Add comprehensive image extraction support
- Add polars DataFrame and PIL Image serialization for API responses
- Improve test coverage across core modules

### Fixed

- Resolve mypy type errors in CI
- Remove unused flame_config parameter from profile_benchmark
- Resolve CI formatting issues by ignoring generated AGENTS.md
- Resolve pre-commit formatting issues and ruff violations
- Resolve all test failures and achieve full test suite compliance
- Add polars DataFrame and PIL Image serialization for API responses
- Resolve TypeError unhashable type dict in API config merging
- Address linting and type issues from PR #130 review

### Changed

- Apply ruff formatting across the codebase

### Documentation

- Add comprehensive image extraction documentation
- Update documentation to use ImageOCRConfig instead of deprecated fields

### Cleanup

- Remove INTERFACE_PARITY.md causing mdformat issues

---

## [3.14.0] - 2025-09-13

### Added

- Comprehensive DPI configuration system for OCR processing with fine-grained control over resolution settings

### Changed

- Enhanced API with 1GB upload limit and comprehensive OpenAPI documentation
- Completed pandas to polars migration across entire codebase for improved performance and memory efficiency

### Fixed

- Improved lcov coverage combining robustness in CI pipeline
- DPI configuration tests moved to proper test directory for correct CI coverage calculation
- Pre-commit formatting issues resolved
- PDF content handling differences between CI and local tests
- PaddleOCR test isolation and fixture path corrections

---

## [3.13.0] - 2025-09-04

### Added

- Runtime configuration API with query parameters and header support for flexible request handling
- Comprehensive runtime configuration documentation with practical examples
- OCR caching system for EasyOCR and PaddleOCR backends to improve performance

### Changed

- Replaced pandas with polars for table extraction (significant performance improvement)
- Consolidated benchmarks CLI into unified interface with improved structure
- Converted all class-based tests to function-based tests for consistency
- Restructured benchmarks package layout for better organization
- Removed all module-level docstrings from Python files for cleaner codebase

### Fixed

- MyPy errors and type annotations throughout codebase
- Tesseract TSV output format and table extraction implementation
- UTF-8 encoding handling across document processing pipeline
- Config loading with proper error handling and validation
- HTML-to-Markdown configuration now externalized for flexibility
- All ruff and mypy linting errors resolved
- Test failures in CI environment
- Regression in PDF extraction and XLS file handling
- Subprocess error analysis for CalledProcessError handling
- Empty DataFrames in GMFT table extraction (pandas.errors.EmptyDataError prevention)

### Performance

- Optimized PDF processing and HTML conversion performance
- Improved TSV integration test quality with concrete assertions

---

## [3.12.0] - 2025-08-30

### Added

- Docker E2E testing infrastructure with multi-image matrix strategy
- Multilingual OCR support to Docker images with flexible backend selection
- Docker documentation with clarity on image contents, sizes, and OCR backend selection

### Changed

- Simplified Docker images to base and core variants (removed intermediate images)
- Docker strategy updated to 2-image approach for optimized distribution
- Docker E2E tests aligned with CI patterns using matrix strategy for parallel execution
- Improved Docker workflow with manual trigger options for flexibility

### Fixed

- Docker image naming conventions (kreuzberg-core:latest)
- Boolean condition syntax in Docker workflow
- Docker permissions and documentation accuracy
- Docker E2E test failure detection (ensures proper test exit code propagation)
- Docker workflow disk space management and optimization
- Grep exit code handling in Docker test workflows
- Naming conflict in CLI config command
- grep exit code failures in shell scripts
- EasyOCR test image selection (use text-containing image instead of flower)
- Test fixture image file naming for clarity

### Infrastructure

- Enhanced disk space cleanup in Docker E2E workflow
- Improved Docker workflow disk space management
- Updated test-docker-builds.yml for new image naming scheme

---

## [3.11.1] - 2025-08-13

### Fixed

- EasyOCR device-related parameters removed from readtext() calls
- Numpy import optimization: only import numpy inside process_image_sync rather than at module level to improve startup time
- Table attribute access in documentation
- Pre-commit formatting for table documentation

### Infrastructure

- Updated uv.lock after version bump
- AI-rulez pre-commit hooks compatibility with Go setup in CI workflow
- Temporary ai-rulez hooks disabled to unblock CI
- Reverted to ai-ruyles v1.4.3 with Go setup for CI stability
- Dependency updates including amannn/action-semantic-pull-request v5→v6 and actions/checkout v4→v5

---

## [3.11.0] - 2025-08-01

### Added

- Comprehensive tests for Tesseract OCR backend with edge case coverage
- Comprehensive tests for API module with full functionality coverage
- Comprehensive tests for config and image extractor modules
- Comprehensive Click-based tests for CLI module with proper command testing
- Comprehensive tests for structured data extractor module
- Comprehensive tests for extraction and document classification modules
- Comprehensive tests for pandoc extractor module
- Comprehensive tests for email extractor module
- Comprehensive tests for _config.py module
- Comprehensive tests for _utils._errors module with error handling validation
- Comprehensive tests for spreadsheet metadata extraction
- Retry mechanisms for CI test reliability to handle transient failures

### Changed

- Implemented Python 3.10+ syntax optimizations (match/case, union types where applicable)
- Merged comprehensive test files with regular test files for consolidated coverage
- Optimized pragma no cover usage to reduce false negatives
- Converted test classes to functional tests for better pytest compatibility
- Updated coverage requirements documentation from 95% to 85%

### Fixed

- Linting issues in merged comprehensive test files
- Image extractor async delegation tests
- CI test failures with improved retry mechanisms
- Timezone assertion in spreadsheet metadata test
- MyPy errors in test files and config calls
- ExceptionGroup import errors for Python 3.10+ compatibility
- Syntax errors in test files
- Bug in _parse_date_string with proper test mocks
- Pre-commit formatting issues
- DeepSource coverage reporting with restructured CI workflow

### Infrastructure

- Updated uv.lock to revision 3 to fix CI validation issues
- Fixed CI workflow structure for improved DeepSource coverage reporting
- Coverage upload now only triggers on push events (not pull requests)
- Version bump to 3.10.1 with lock file updates

---

## [3.10.0] - 2025-07-29

### Added

- PDF password support through new crypto extra feature

### Documentation

- Updated quick start section to use available configuration options

---

## [3.9.0] - 2025-07-17

### Added

- Initial release of v3.9.0 series with foundational features

---

## [3.8.0] - 2025-07-16

### Added

- Foundation for v3.8.0 release

---

## [3.7.0] - 2025-07-11

### Added

- MCP server for AI integration enabling Claude integration with document extraction capabilities
- MCP server configuration and optional dependencies support

### Fixed

- Chunk parameters adjusted to prevent overlap validation errors
- MCP server linting issues resolved
- HTML test compatibility with html-to-markdown v1.6.0 behavior

### Changed

- MCP server tests converted to function-based format
- MCP server configuration documentation clarified

---

## [3.6.0] - 2025-07-04

### Added

- Language detection functionality integrated into extraction pipeline

### Fixed

- Entity extraction migration from gliner to spaCy completed
- Docker workflow reliability improved with updated ai-rulez configuration
- Optional imports refactored to use classes for better type handling
- Validation and post-process logic abstracted into helper functions for sync and async paths

### Changed

- spaCy now used for entity extraction replacing gliner
- Optional imports moved inside functions for proper error handling
- Entity and keyword extraction validation/post-processing refactored

---

## [3.5.0] - 2025-07-04

### Added

- Language detection functionality with configurable backends
- Performance optimization guidelines and documentation
- Full synchronous support for PaddleOCR and EasyOCR backends
- Docker documentation and Python 3.10+ compatibility
- Benchmarks submodule integration

### Fixed

- Flaky OCR tests marked as xfail in CI environments
- Typing errors resolved across codebase
- Chunking default configuration reverted to stable settings
- PaddleOCR sync implementation updated to correct v3.x API format
- Python 3.9 support dropped and version bumped to v3.4.2
- Docker workflow version detection and dependencies fixed
- Docs workflow configuration and Docker manual dispatch enabled
- CI documentation dependencies resolved

### Changed

- Default configurations optimized for modern document extraction
- Tesseract OCR configuration optimized based on benchmark analysis
- Documentation link moved to top of README
- Python 3.10+ now required (3.9 support dropped)

---

## [3.4.0] - 2025-07-03

### Added

- API support with Litestar framework for web-based document extraction
- API tests aligned with best practices
- EasyOCR and GMFT Docker build variants
- Docker support with comprehensive CI integration
- API and Docker documentation

### Fixed

- 'all' extra dependency configuration simplified
- Docker workflow extras configuration corrected
- Race condition in GMFT caching tests resolved
- Race condition in Tesseract caching tests resolved
- Docker Hub repository configuration fixed
- CLI integration test using tmp_path fixture for isolation
- gmft module mypy type errors resolved

### Changed

- Docker module migrated to api module with Litestar framework
- Docker workflow matrix simplified
- Performance documentation enhanced with competitive benchmarks

---

## [3.3.0] - 2025-06-23

### Added

- Isolated process wrapper for GMFT table extraction
- Comprehensive statistical benchmarks proving msgpack superiority
- Comprehensive CLI support with Click framework
- Comprehensive benchmarking suite for sync vs async performance
- Pure synchronous extractors without anyio dependencies
- Document-level caching with simplified concurrency handling
- Per-file locks and parallel batch processing for sync performance
- Python version matrix CI testing (3.9-3.13)
- Thread lock for pypdfium2 to prevent macOS segfaults

### Fixed

- All remaining mypy and pydoclint issues ensuring CI passes
- Python 3.13 mock compatibility issues in process pool tests
- ExceptionGroup compatibility in sync tests
- GMFT config updated and test failures resolved
- All Windows-specific test failures in multiprocessing and utils modules
- Windows compatibility issues resolved
- Test coverage improvements from 76% to 83%
- Rust formatting issues with edition 2024 let-chain syntax
- File existence validation added to extraction functions
- Python linting issues (ruff complexity, mypy type parameters) resolved
- All clippy warnings resolved
- C# generated file formatting fixed

### Changed

- msgspec JSON replaced with msgpack for 5x faster cache serialization
- Caching migrated from msgspec JSON to msgpack for improved performance
- Async performance optimized with semaphore-based concurrency control
- Sync performance optimized with per-file locks and parallel batch processing
- Code formatting cleanup and non-essential comments removed
- Benchmark workflows removed from CI
- Python version cache key updated to include full version

---

## [3.2.0] - 2025-06-23

### Added

- GPU acceleration support for enhanced OCR and ML operations

### Fixed

- EasyOCR byte string issues resolved
- Pandoc version issues fixed
- PaddleOCR configuration updated for optimal performance
- Multiple language support added to EasyOCR

### Changed

- playa-pdf pinned to version 0.4.3 for stability
- PaddleOCR configuration optimized
- Dependencies updated to latest compatible versions
- Pandoc version string resolution improved

---

## [3.1.0] - 2025-03-28

### Added

- GMFT (Give Me Formatted Tables) support for vision-based table extraction

### Fixed

- Bundling issue corrected
- Wrong link in README fixed
- Dependencies updated and GMFT testing issues resolved

### Changed

- Image extraction now non-optional in results
- Test imports and structure updated
- Concurrency test added for validation

---

## [3.0.0] - 2025-03-23

### Added

- Chunking functionality for document segmentation
- Extractor registry for managing format-specific extractors
- Hooks system for pre/post-processing
- OCR backend abstraction with EasyOCR and PaddleOCR support
- OCR configuration system
- Multiple language support in OCR backends
- Comprehensive documentation

### Fixed

- Pre-commit hooks configuration
- Windows error message handling
- PaddleOCR integration issues
- Dependencies updated to stable versions

### Changed

- Structure refactored for improved organization
- Metadata extraction approach updated
- OCR integration with configurable backends
- Documentation added and improved

---

## See Also

- [Configuration Reference](reference/configuration.md) - Detailed configuration options
- [Migration Guide](migration/v3-to-v4.md) - v3 to v4 migration instructions
- [Format Support](reference/formats.md) - Supported file formats
- [Extraction Guide](guides/extraction.md) - Extraction examples

[4.0.0-rc.7]: https://github.com/kreuzberg-dev/kreuzberg/compare/v4.0.0-rc.6...v4.0.0-rc.7
[4.0.0-rc.6]: https://github.com/kreuzberg-dev/kreuzberg/compare/v4.0.0-rc.5...v4.0.0-rc.6
[4.0.0-rc.5]: https://github.com/kreuzberg-dev/kreuzberg/compare/v4.0.0-rc.4...v4.0.0-rc.5
[4.0.0-rc.4]: https://github.com/kreuzberg-dev/kreuzberg/compare/v4.0.0-rc.3...v4.0.0-rc.4
[4.0.0-rc.3]: https://github.com/kreuzberg-dev/kreuzberg/compare/v4.0.0-rc.2...v4.0.0-rc.3
[4.0.0-rc.2]: https://github.com/kreuzberg-dev/kreuzberg/compare/v4.0.0-rc.1...v4.0.0-rc.2
[4.0.0-rc.1]: https://github.com/kreuzberg-dev/kreuzberg/compare/v4.0.0-rc.0...v4.0.0-rc.1
[4.0.0-rc.0]: https://github.com/kreuzberg-dev/kreuzberg/compare/v3.21.0...v4.0.0-rc.0
[3.22.0]: https://github.com/kreuzberg-dev/kreuzberg/compare/v3.21.0...v3.22.0
[3.21.0]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v3.21.0
[3.20.2]: https://github.com/kreuzberg-dev/kreuzberg/compare/v3.20.1...v3.20.2
[3.20.1]: https://github.com/kreuzberg-dev/kreuzberg/compare/v3.20.0...v3.20.1
[3.20.0]: https://github.com/kreuzberg-dev/kreuzberg/compare/v3.19.1...v3.20.0
[3.19.1]: https://github.com/kreuzberg-dev/kreuzberg/compare/v3.19.0...v3.19.1
[3.19.0]: https://github.com/kreuzberg-dev/kreuzberg/compare/v3.18.0...v3.19.0
[3.18.0]: https://github.com/kreuzberg-dev/kreuzberg/compare/v3.17.0...v3.18.0
[3.17.0]: https://github.com/kreuzberg-dev/kreuzberg/compare/v3.16.0...v3.17.0
[3.16.0]: https://github.com/kreuzberg-dev/kreuzberg/compare/v3.15.0...v3.16.0
[3.15.0]: https://github.com/kreuzberg-dev/kreuzberg/compare/v3.14.0...v3.15.0
[3.14.0]: https://github.com/kreuzberg-dev/kreuzberg/compare/v3.13.0...v3.14.0
[3.13.0]: https://github.com/kreuzberg-dev/kreuzberg/compare/v3.12.0...v3.13.0
[3.12.0]: https://github.com/kreuzberg-dev/kreuzberg/compare/v3.11.1...v3.12.0
[3.11.1]: https://github.com/kreuzberg-dev/kreuzberg/compare/v3.11.0...v3.11.1
[3.11.0]: https://github.com/kreuzberg-dev/kreuzberg/compare/v3.10.0...v3.11.0
[3.10.0]: https://github.com/kreuzberg-dev/kreuzberg/compare/v3.9.0...v3.10.0
[3.9.0]: https://github.com/kreuzberg-dev/kreuzberg/compare/v3.8.0...v3.9.0
[3.8.0]: https://github.com/kreuzberg-dev/kreuzberg/compare/v3.7.0...v3.8.0
[3.7.0]: https://github.com/kreuzberg-dev/kreuzberg/compare/v3.6.0...v3.7.0
[3.6.0]: https://github.com/kreuzberg-dev/kreuzberg/compare/v3.5.0...v3.6.0
[3.5.0]: https://github.com/kreuzberg-dev/kreuzberg/compare/v3.4.0...v3.5.0
[3.4.0]: https://github.com/kreuzberg-dev/kreuzberg/compare/v3.3.0...v3.4.0
[3.3.0]: https://github.com/kreuzberg-dev/kreuzberg/compare/v3.2.0...v3.3.0
[3.2.0]: https://github.com/kreuzberg-dev/kreuzberg/compare/v3.1.0...v3.2.0
[3.1.0]: https://github.com/kreuzberg-dev/kreuzberg/compare/v3.0.0...v3.1.0
[3.0.0]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v3.0.0
