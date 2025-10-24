# Kreuzberg Documentation Coverage Audit

This file tracks the coverage of all public APIs against the documentation. Use this as a checklist to ensure comprehensive documentation across all languages and features.

**Legend:**
- ✅ Fully documented (docstrings + usage guide + examples)
- 📝 Partially documented (missing examples or usage guide)
- ❌ Not documented
- N/A Not applicable

**Last Updated:** 2025-10-24

---

## 1. Core Rust API (crates/kreuzberg)

### 1.1 Main Extraction Functions (`src/core/extractor.rs`)

| Function | Rust Docs | Usage Guide | Examples | Status |
|----------|-----------|-------------|----------|--------|
| `extract_file()` | ✅ | ✅ (docs/examples/rust.md) | ✅ | ✅ |
| `extract_file_sync()` | ✅ | ✅ (docs/examples/rust.md) | ✅ | ✅ |
| `extract_bytes()` | ✅ | ✅ (docs/examples/rust.md) | ✅ | ✅ |
| `extract_bytes_sync()` | ✅ | ✅ (docs/examples/rust.md) | ✅ | ✅ |
| `batch_extract_file()` | ✅ | ✅ (docs/examples/rust.md) | ✅ | ✅ |
| `batch_extract_file_sync()` | ✅ | ✅ (docs/examples/rust.md) | ✅ | ✅ |
| `batch_extract_bytes()` | ✅ | ✅ (docs/examples/rust.md) | ✅ | ✅ |
| `batch_extract_bytes_sync()` | ✅ | ✅ (docs/examples/rust.md) | ✅ | ✅ |

**Notes:**
- Module-level docs: Excellent (lines 1-12)
- All functions have comprehensive doc comments with:
  - Clear parameter descriptions
  - Return value documentation
  - Error documentation with specific error types
  - Examples in doc comments
  - Performance notes (e.g., global runtime usage)
  - Safety comments where applicable

### 1.2 Configuration (`src/core/config.rs`)

| Config Type | Rust Docs | Usage Guide | Examples | Status |
|-------------|-----------|-------------|----------|--------|
| `ExtractionConfig` | ✅ | ✅ (CLI usage.md) | ✅ | ✅ |
| `OcrConfig` | ✅ | ✅ (docs/concepts/ocr.md) | ✅ | ✅ |
| `PdfConfig` | ✅ | ✅ (CLI usage.md) | ✅ | ✅ |
| `ChunkingConfig` | ✅ | ✅ (CLI usage.md) | ✅ | ✅ |
| `TokenReductionConfig` | ✅ | ✅ (CLI usage.md) | ✅ | ✅ |
| `LanguageDetectionConfig` | ✅ | ✅ (CLI usage.md) | ✅ | ✅ |
| `ImageExtractionConfig` | ✅ | ✅ (CLI usage.md) | ✅ | ✅ |
| `PostProcessorConfig` | ✅ | ✅ (docs/concepts/text-processing.md) | ✅ | ✅ |

**Notes:**
- Module-level docs: Good (lines 1-4)
- `ExtractionConfig`: Excellent docs with example (lines 10-25), all fields documented
- All config structs have Serde derive for TOML/YAML/JSON loading
- Field-level documentation with `#[serde(default)]` attributes documented
- Config file discovery documented in module

### 1.3 MIME Type Detection (`src/core/mime.rs`)

| Function/Constant | Rust Docs | Usage Guide | Examples | Status |
|-------------------|-----------|-------------|----------|--------|
| `detect_mime_type()` | ❌ | ✅ (docs/concepts/extractors.md) | N/A | 📝 |
| `validate_mime_type()` | ❌ | ✅ (docs/concepts/extractors.md) | N/A | 📝 |
| `detect_or_validate()` | ❌ | ✅ (docs/concepts/extractors.md) | N/A | 📝 |
| MIME type constants | ✅ | ✅ (docs/concepts/extractors.md) | N/A | ✅ |

**Notes:**
- Module-level docs: Good (lines 1-4)
- Constants well-defined and exported
- **NEEDS IMPROVEMENT**: Public functions lack doc comments
- **ACTION ITEM**: Add function-level documentation with examples

### 1.4 Types (`src/types.rs`)

| Type | Rust Docs | Usage Guide | Examples | Status |
|------|-----------|-------------|----------|--------|
| `ExtractionResult` | ✅ | ✅ (examples) | ✅ | ✅ |
| `Metadata` | ✅ | ✅ (examples) | ✅ | ✅ |
| `Table` | 📝 | ✅ (CLI usage.md) | ✅ | 📝 |
| Various metadata structs | 📝 | ✅ (per extractor) | ✅ | 📝 |

**Notes:**
- `ExtractionResult` has doc comment (line 12-14)
- `Metadata` has doc comment (lines 33-37)
- Many metadata structs lack doc comments (e.g., `ExcelMetadata`, `EmailMetadata`, `XmlMetadata`)
- **ACTION ITEM**: Add doc comments to all public metadata types with field descriptions
- **TODO comment on line 10**: "sort types meant for external consumption alphabetically and add doc strings as required"

### 1.5 Plugin System (`src/plugins/`)

| Component | Rust Docs | Usage Guide | Examples | Status |
|-----------|-----------|-------------|----------|--------|
| `Plugin` trait | ✅ | ✅ (docs/plugins/) | ✅ | ✅ |
| `DocumentExtractor` trait | ✅ | ✅ (docs/plugins/) | ✅ | ✅ |
| `OcrBackend` trait | ✅ | ✅ (docs/plugins/) | ✅ | ✅ |
| `PostProcessor` trait | ✅ | ✅ (docs/plugins/) | ✅ | ✅ |
| `Validator` trait | ✅ | ✅ (docs/plugins/) | ✅ | ✅ |
| `get_document_extractor_registry()` | ✅ | ✅ (docs/plugins/) | ✅ | ✅ |
| `get_ocr_backend_registry()` | ✅ | ✅ (docs/plugins/) | ✅ | ✅ |
| `get_post_processor_registry()` | ✅ | ✅ (docs/plugins/) | ✅ | ✅ |
| `get_validator_registry()` | ✅ | ✅ (docs/plugins/) | ✅ | ✅ |

**Notes:**
- Module-level docs: Excellent (lines 1-100+) with comprehensive overview
- Includes lifecycle patterns, language support notes, full working examples
- Plugin architecture fully documented with Arc pattern
- Usage guides in docs/plugins/ cover Rust, Python, and TypeScript implementations

### 1.6 Error Types (`src/error.rs`)

| Type | Rust Docs | Usage Guide | Examples | Status |
|------|-----------|-------------|----------|--------|
| `KreuzbergError` enum | ❌ | ✅ (error handling guide) | ✅ | 📝 |
| `Result<T>` type alias | ❌ | N/A | N/A | 📝 |

**Notes:**
- **NEEDS IMPROVEMENT**: No module-level or enum-level documentation
- Error variants use `thiserror` with good display messages
- All variants have proper `#[source]` attributes for error chaining
- **ACTION ITEM**: Add module docs explaining error handling philosophy
- **ACTION ITEM**: Add enum-level documentation with examples

---

## 2. Python API (`packages/python/kreuzberg/`)

### 2.1 Package Documentation

| File | Module Docstring | Usage Guide | Examples | Status |
|------|------------------|-------------|----------|--------|
| `__init__.py` | ✅ | ✅ (docs/examples/python.md) | ✅ | ✅ |
| `types.py` | ✅ | ✅ (docs/examples/python.md) | ✅ | ✅ |
| `exceptions.py` | ✅ | ✅ (docs/examples/python.md) | ✅ | ✅ |

**Notes:**
- `__init__.py` has comprehensive module docstring with:
  - Architecture explanation (Rust core + Python wrapper)
  - Python-specific features listed
  - Custom PostProcessor creation example
- All type classes have docstrings
- All exception classes have excellent docstrings with examples
- **EXCELLENT OVERALL**: Python API is well-documented at package level

### 2.2 Main Extraction Functions (via bindings)

| Function | Python Docs | Usage Guide | Examples | Status |
|----------|-------------|-------------|----------|--------|
| `extract_file()` | ✅ (via __init__.py) | ✅ (docs/examples/python.md) | ✅ | ✅ |
| `extract_file_sync()` | ✅ (via __init__.py) | ✅ (docs/examples/python.md) | ✅ | ✅ |
| `extract_bytes()` | ✅ (via __init__.py) | ✅ (docs/examples/python.md) | ✅ | ✅ |
| `extract_bytes_sync()` | ✅ (via __init__.py) | ✅ (docs/examples/python.md) | ✅ | ✅ |
| `batch_extract_files()` | ✅ (via __init__.py) | ✅ (docs/examples/python.md) | ✅ | ✅ |
| `batch_extract_files_sync()` | ✅ (via __init__.py) | ✅ (docs/examples/python.md) | ✅ | ✅ |
| `batch_extract_bytes()` | ✅ (via __init__.py) | ✅ (docs/examples/python.md) | ✅ | ✅ |
| `batch_extract_bytes_sync()` | ✅ (via __init__.py) | ✅ (docs/examples/python.md) | ✅ | ✅ |

**Notes:**
- Functions are imported from Rust bindings
- Module docstring covers usage patterns
- Full examples in docs/examples/python.md

### 2.3 Configuration and Types

| Class | Python Docs | Usage Guide | Examples | Status |
|-------|-------------|-------------|----------|--------|
| All config classes | ✅ (from Rust) | ✅ (CLI usage.md) | ✅ | ✅ |
| All metadata types | ✅ (TypedDict with docs) | ✅ (examples) | ✅ | ✅ |

### 2.4 Exceptions (`exceptions.py`)

| Exception | Docstring | Usage Guide | Examples | Status |
|-----------|-----------|-------------|----------|--------|
| `KreuzbergError` | ✅ | ✅ (CLAUDE.md) | ✅ | ✅ |
| `ValidationError` | ✅ | ✅ (CLAUDE.md) | ✅ | ✅ |
| `ParsingError` | ✅ | ✅ (CLAUDE.md) | ✅ | ✅ |
| `OCRError` | ✅ | ✅ (CLAUDE.md) | ✅ | ✅ |
| `MissingDependencyError` | ✅ | ✅ (CLAUDE.md) | ✅ | ✅ |

**Notes:**
- Each exception has comprehensive docstring with example
- `MissingDependencyError` has `.create_for_package()` factory method with full docs

### 2.5 Python-Specific Features

| Feature | Module Docs | Usage Guide | Examples | Status |
|---------|-------------|-------------|----------|--------|
| EasyOCR backend | ✅ | ✅ (docs/concepts/ocr.md) | ✅ | ✅ |
| PaddleOCR backend | ✅ | ✅ (docs/concepts/ocr.md) | ✅ | ✅ |
| PostProcessor protocol | ✅ | ✅ (docs/plugins/python-postprocessor.md) | ✅ | ✅ |
| API Server (Litestar) | ✅ | ✅ (docs/concepts/server.md) | ✅ | ✅ |
| MCP Server | ✅ | ✅ (docs/concepts/server.md) | ✅ | ✅ |
| CLI proxy | ✅ | ✅ (docs/cli/usage.md) | ✅ | ✅ |

**Notes:**
- All Python-specific features have module docstrings
- OCR backends have language support constants documented

---

## 3. TypeScript API (`packages/typescript/`)

### 3.1 Package Documentation

| File | JSDoc/TSDoc | Usage Guide | Examples | Status |
|------|-------------|-------------|----------|--------|
| `index.ts` | ✅ | ✅ (docs/examples/typescript.md) | ✅ | ✅ |
| `types.ts` | ✅ | ✅ (docs/examples/typescript.md) | ✅ | ✅ |

**Notes:**
- `index.ts` has comprehensive JSDoc with:
  - API usage recommendations (batch vs single extraction)
  - Supported formats list
  - Complete usage examples
- `types.ts` has module-level documentation
- **EXCELLENT OVERALL**: TypeScript API is well-documented with TSDoc

### 3.2 Main Extraction Functions

| Function | TSDoc | Usage Guide | Examples | Status |
|----------|-------|-------------|----------|--------|
| `extractFile()` | ✅ (via index.ts) | ✅ (docs/examples/typescript.md) | ✅ | ✅ |
| `extractFileSync()` | ✅ (via index.ts) | ✅ (docs/examples/typescript.md) | ✅ | ✅ |
| `extractBytes()` | ✅ (via index.ts) | ✅ (docs/examples/typescript.md) | ✅ | ✅ |
| `extractBytesSync()` | ✅ (via index.ts) | ✅ (docs/examples/typescript.md) | ✅ | ✅ |
| `batchExtractFiles()` | ✅ (via index.ts) | ✅ (docs/examples/typescript.md) | ✅ | ✅ |
| `batchExtractFilesSync()` | ✅ (via index.ts) | ✅ (docs/examples/typescript.md) | ✅ | ✅ |
| `batchExtractBytes()` | ✅ (via index.ts) | ✅ (docs/examples/typescript.md) | ✅ | ✅ |
| `batchExtractBytesSync()` | ✅ (via index.ts) | ✅ (docs/examples/typescript.md) | ✅ | ✅ |

**Notes:**
- Module-level JSDoc covers API usage patterns
- Includes code examples in JSDoc
- Full examples in docs/examples/typescript.md

### 3.3 Configuration Interfaces

| Interface | TSDoc | Usage Guide | Examples | Status |
|-----------|-------|-------------|----------|--------|
| `ExtractionConfig` | ✅ | ✅ (CLI usage.md) | ✅ | ✅ |
| `OcrConfig` | ✅ | ✅ (docs/concepts/ocr.md) | ✅ | ✅ |
| `PdfConfig` | ✅ | ✅ (CLI usage.md) | ✅ | ✅ |
| `ChunkingConfig` | ✅ | ✅ (CLI usage.md) | ✅ | ✅ |
| `TokenReductionConfig` | ✅ | ✅ (CLI usage.md) | ✅ | ✅ |
| `LanguageDetectionConfig` | ✅ | ✅ (CLI usage.md) | ✅ | ✅ |
| `ImageExtractionConfig` | ✅ | ✅ (CLI usage.md) | ✅ | ✅ |
| `PostProcessorConfig` | ✅ | ✅ (docs/concepts/text-processing.md) | ✅ | ✅ |

**Notes:**
- All interfaces have proper TypeScript typing
- Type definitions mirror Rust structures

### 3.4 Types (`types.ts`)

| Type | TSDoc | Usage Guide | Examples | Status |
|------|-------|-------------|----------|--------|
| `ExtractionResult` | ✅ | ✅ (examples) | ✅ | ✅ |
| `Metadata` | ✅ | ✅ (examples) | ✅ | ✅ |
| `Table` | ✅ | ✅ (CLI usage.md) | ✅ | ✅ |
| Various metadata interfaces | ✅ | ✅ (per extractor) | ✅ | ✅ |

**Notes:**
- Module docstring explains type definitions
- All interfaces properly typed

### 3.5 TypeScript-Specific Features

| Feature | TSDoc | Usage Guide | Examples | Status |
|---------|-------|-------------|----------|--------|
| CLI proxy | ✅ | ✅ (docs/cli/usage.md) | ✅ | ✅ |
| NAPI bindings | ✅ | ✅ (index.ts) | ✅ | ✅ |

---

## 4. CLI Commands (`crates/kreuzberg-cli/`)

| Command | Help Text | Usage Guide | Examples | Status |
|---------|-----------|-------------|----------|--------|
| `kreuzberg extract` | ❌ | ✅ | ✅ | 📝 |
| `kreuzberg serve` | ✅ | ✅ | ✅ | ✅ |
| `kreuzberg mcp` | ✅ | ✅ | ✅ | ✅ |
| `kreuzberg cache stats` | ❌ | ✅ | ✅ | 📝 |
| `kreuzberg cache clear` | ❌ | ✅ | ✅ | 📝 |
| `kreuzberg completion` | ❌ | ✅ | ✅ | 📝 |
| `kreuzberg --version` | ❌ | ✅ | ✅ | 📝 |
| `kreuzberg --help` | ❌ | ✅ | ✅ | 📝 |

---

## 5. Optional Features

### 5.1 OCR Support (`feature = "ocr"`)

| Component | Rust Docs | Usage Guide | Examples | Status |
|-----------|-----------|-------------|----------|--------|
| `src/ocr/processor.rs` | ❌ | ❌ | ❌ | ❌ |
| `src/ocr/tesseract.rs` | ❌ | ❌ | ❌ | ❌ |
| `src/ocr/cache.rs` | ❌ | ❌ | ❌ | ❌ |
| `src/ocr/types.rs` | ❌ | ❌ | ❌ | ❌ |
| Table detection | ❌ | ❌ | ❌ | ❌ |

### 5.2 PDF Support (`feature = "pdf"`)

| Component | Rust Docs | Usage Guide | Examples | Status |
|-----------|-----------|-------------|----------|--------|
| `src/pdf/extraction.rs` | ❌ | ❌ | ❌ | ❌ |
| `src/pdf/image.rs` | ❌ | ❌ | ❌ | ❌ |
| `src/pdf/metadata.rs` | ❌ | ❌ | ❌ | ❌ |
| `src/pdf/types.rs` | ❌ | ❌ | ❌ | ❌ |

### 5.3 Chunking (`feature = "chunking"`)

| Component | Rust Docs | Usage Guide | Examples | Status |
|-----------|-----------|-------------|----------|--------|
| `src/chunking/` module | ❌ | ❌ | ❌ | ❌ |

### 5.4 Language Detection (`feature = "language-detection"`)

| Component | Rust Docs | Usage Guide | Examples | Status |
|-----------|-----------|-------------|----------|--------|
| `src/language_detection.rs` | ❌ | ❌ | ❌ | ❌ |

### 5.5 Keywords Extraction (`feature = "keywords-yake"`, `feature = "keywords-rake"`)

| Component | Rust Docs | Usage Guide | Examples | Status |
|-----------|-----------|-------------|----------|--------|
| `src/keywords/` module | ❌ | ❌ | ❌ | ❌ |

### 5.6 Stopwords (`feature = "stopwords"`)

| Component | Rust Docs | Usage Guide | Examples | Status |
|-----------|-----------|-------------|----------|--------|
| `src/stopwords/` module | ❌ | ❌ | ❌ | ❌ |

### 5.7 API Server (`feature = "api"`)

| Component | Rust Docs | Usage Guide | Examples | Status |
|-----------|-----------|-------------|----------|--------|
| `src/api/` module | ❌ | ✅ | ✅ | 📝 |
| API endpoints | ❌ | ✅ | ✅ | 📝 |

### 5.8 MCP Server (`feature = "mcp"`)

| Component | Rust Docs | Usage Guide | Examples | Status |
|-----------|-----------|-------------|----------|--------|
| `src/mcp/` module | ❌ | ✅ | ✅ | 📝 |
| MCP tools | ❌ | ✅ | ✅ | 📝 |

---

## 6. Format-Specific Extractors

### 6.1 Extraction Functions (`src/extraction/`)

These are low-level parsing functions used by the extractor plugins.

| Module | Module Docs | Function Docs | Usage Guide | Status |
|--------|-------------|---------------|-------------|--------|
| `archive.rs` | ✅ | ✅ (type docs) | ✅ (CLI usage.md) | ✅ |
| `email.rs` | ❌ | ✅ (parse_eml_content) | ✅ (examples) | 📝 |
| `excel.rs` | ❌ | ❌ | ✅ (CLI usage.md) | 📝 |
| `html.rs` | ❌ | ❌ | ✅ (examples) | 📝 |
| `image.rs` | ✅ | ✅ | ✅ (examples) | ✅ |
| `structured.rs` | ❌ | ❌ | ✅ (examples) | 📝 |
| `text.rs` | ❌ | ❌ | ✅ (CLI usage.md) | 📝 |
| `xml.rs` | ❌ | ❌ | ✅ (CLI usage.md) | 📝 |
| `libreoffice.rs` | ❌ | ❌ | ✅ (CLAUDE.md) | 📝 |
| `pptx.rs` | ❌ | ❌ | ✅ (examples) | 📝 |
| `table.rs` | ❌ | ❌ | ✅ (CLI usage.md) | 📝 |

**Notes:**
- archive.rs and image.rs have good documentation
- Most extraction modules lack module-level docs
- **ACTION ITEM**: Add module docs to text.rs, xml.rs, structured.rs, email.rs, excel.rs, html.rs, pptx.rs

### 6.2 Extractor Plugins (`src/extractors/`)

These are the plugin implementations that use the extraction functions.

| Extractor | Module Docs | Struct Docs | Usage Guide | Status |
|-----------|-------------|-------------|-------------|--------|
| `PlainTextExtractor` (text.rs) | ✅ | ✅ | ✅ (CLI usage.md) | ✅ |
| `MarkdownExtractor` (text.rs) | ✅ | ✅ | ✅ (CLI usage.md) | ✅ |
| `ExcelExtractor` (excel.rs) | ✅ | ✅ | ✅ (CLI usage.md) | ✅ |
| `EmailExtractor` (email.rs) | ❌ | ❌ | ✅ (examples) | 📝 |
| `HtmlExtractor` (html.rs) | ❌ | ❌ | ✅ (examples) | 📝 |
| `ImageExtractor` (image.rs) | ❌ | ❌ | ✅ (examples) | 📝 |
| `StructuredDataExtractor` (structured.rs) | ❌ | ❌ | ✅ (examples) | 📝 |
| `XmlExtractor` (xml.rs) | ❌ | ❌ | ✅ (CLI usage.md) | 📝 |
| `ArchiveExtractor` (archive.rs) | ❌ | ❌ | ✅ (examples) | 📝 |

**Notes:**
- text.rs and excel.rs extractors have excellent documentation
- **ACTION ITEM**: Add module and struct docs to remaining extractor plugins

---

## 7. Documentation Infrastructure

| Component | Status | Notes |
|-----------|--------|-------|
| README.md | ✅ | Complete |
| API Reference (Rust) | ❌ | Need cargo doc |
| API Reference (Python) | ❌ | Need Sphinx/mkdocs |
| API Reference (TypeScript) | ❌ | Need TypeDoc |
| Getting Started Guide | ❌ | Need comprehensive guide |
| Architecture Guide | 📝 | Partial in CLAUDE.md |
| Plugin Development Guide | ❌ | Not started |
| Migration Guide | ❌ | Not started |

---

## 8. Action Items by Priority

### Priority 1: Core API Documentation (Rust)
- [ ] Document main extraction functions in `src/core/extractor.rs`
- [ ] Document configuration types in `src/core/config.rs`
- [ ] Document MIME detection in `src/core/mime.rs`
- [ ] Document types in `src/types.rs`
- [ ] Document error types in `src/error.rs`

### Priority 2: Python Bindings
- [ ] Add comprehensive docstrings to all public functions
- [ ] Document Python-specific OCR backends (EasyOCR, PaddleOCR)
- [ ] Create Python API reference in docs/

### Priority 3: TypeScript Bindings
- [ ] Add TSDoc comments to all public functions
- [ ] Create TypeScript API reference
- [ ] Add usage examples

### Priority 4: Feature-Specific Documentation
- [ ] OCR module documentation
- [ ] PDF module documentation
- [ ] Chunking feature documentation
- [ ] Language detection feature documentation
- [ ] Keywords extraction feature documentation

### Priority 5: Format Extractors
- [ ] Document each extractor module
- [ ] Add examples for each format

### Priority 6: Infrastructure
- [ ] Set up cargo doc publishing
- [ ] Set up Python API docs (Sphinx)
- [ ] Set up TypeScript API docs (TypeDoc)
- [ ] Create comprehensive getting started guide

---

## Notes

- Use this file as a living document throughout the audit
- Update status as documentation is added
- Link to specific documentation pages once created
- Track feedback and areas needing improvement

---

## Workflow

1. **For each API item:**
   - Add Rust doc comments (`///` for items, `//!` for modules)
   - Add examples in doc comments
   - Update usage guide in docs/
   - Add to API reference
   - Update status in this file

2. **For each language binding:**
   - Ensure parity with Rust docs
   - Add language-specific examples
   - Update language-specific guides

3. **For each feature:**
   - Document configuration options
   - Add usage examples
   - Document limitations and requirements
   - Add troubleshooting guide
