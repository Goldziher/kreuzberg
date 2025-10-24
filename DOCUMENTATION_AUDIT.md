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

### 2.1 Main Extraction Functions (via bindings)

| Function | Python Docs | Usage Guide | Examples | Status |
|----------|-------------|-------------|----------|--------|
| `extract_file()` | ❌ | ❌ | ❌ | ❌ |
| `extract_file_sync()` | ❌ | ❌ | ❌ | ❌ |
| `extract_bytes()` | ❌ | ❌ | ❌ | ❌ |
| `extract_bytes_sync()` | ❌ | ❌ | ❌ | ❌ |
| `batch_extract_file()` | ❌ | ❌ | ❌ | ❌ |
| `batch_extract_file_sync()` | ❌ | ❌ | ❌ | ❌ |
| `batch_extract_bytes()` | ❌ | ❌ | ❌ | ❌ |
| `batch_extract_bytes_sync()` | ❌ | ❌ | ❌ | ❌ |

### 2.2 Configuration Classes

| Class | Python Docs | Usage Guide | Examples | Status |
|-------|-------------|-------------|----------|--------|
| `ExtractionConfig` | ❌ | ❌ | ❌ | ❌ |
| `OcrConfig` | ❌ | ❌ | ❌ | ❌ |
| `PdfConfig` | ❌ | ❌ | ❌ | ❌ |
| `ChunkingConfig` | ❌ | ❌ | ❌ | ❌ |
| `TokenReductionConfig` | ❌ | ❌ | ❌ | ❌ |
| `LanguageDetectionConfig` | ❌ | ❌ | ❌ | ❌ |
| `ImageExtractionConfig` | ❌ | ❌ | ❌ | ❌ |
| `PostProcessorConfig` | ❌ | ❌ | ❌ | ❌ |

### 2.3 Types (`types.py`)

| Type | Python Docs | Usage Guide | Examples | Status |
|------|-------------|-------------|----------|--------|
| `ExtractionResult` | ❌ | ❌ | ❌ | ❌ |
| `ExtractionMetadata` | ❌ | ❌ | ❌ | ❌ |
| `TableData` | ❌ | ❌ | ❌ | ❌ |
| `ChunkData` | ❌ | ❌ | ❌ | ❌ |

### 2.4 Exceptions (`exceptions.py`)

| Exception | Python Docs | Usage Guide | Examples | Status |
|-----------|-------------|-------------|----------|--------|
| `KreuzbergError` | ❌ | ❌ | ❌ | ❌ |
| `ValidationError` | ❌ | ❌ | ❌ | ❌ |
| `ParsingError` | ❌ | ❌ | ❌ | ❌ |
| `OCRError` | ❌ | ❌ | ❌ | ❌ |
| `MissingDependencyError` | ❌ | ❌ | ❌ | ❌ |

### 2.5 Python-Specific Features

| Feature | Python Docs | Usage Guide | Examples | Status |
|---------|-------------|-------------|----------|--------|
| EasyOCR backend | ❌ | ❌ | ❌ | ❌ |
| PaddleOCR backend | ❌ | ❌ | ❌ | ❌ |
| API Server (Litestar) | ✅ | ✅ | ✅ | ✅ |
| MCP Server | ✅ | ✅ | ✅ | ✅ |
| CLI proxy | ✅ | ✅ | ✅ | ✅ |

---

## 3. TypeScript API (`packages/typescript/`)

### 3.1 Main Extraction Functions

| Function | TypeScript Docs | Usage Guide | Examples | Status |
|----------|-----------------|-------------|----------|--------|
| `extractFile()` | ❌ | ❌ | ❌ | ❌ |
| `extractFileSync()` | ❌ | ❌ | ❌ | ❌ |
| `extractBytes()` | ❌ | ❌ | ❌ | ❌ |
| `extractBytesSync()` | ❌ | ❌ | ❌ | ❌ |
| `batchExtractFile()` | ❌ | ❌ | ❌ | ❌ |
| `batchExtractFileSync()` | ❌ | ❌ | ❌ | ❌ |
| `batchExtractBytes()` | ❌ | ❌ | ❌ | ❌ |
| `batchExtractBytesSync()` | ❌ | ❌ | ❌ | ❌ |

### 3.2 Configuration Interfaces

| Interface | TypeScript Docs | Usage Guide | Examples | Status |
|-----------|-----------------|-------------|----------|--------|
| `ExtractionConfig` | ❌ | ❌ | ❌ | ❌ |
| `OcrConfig` | ❌ | ❌ | ❌ | ❌ |
| `PdfConfig` | ❌ | ❌ | ❌ | ❌ |
| `ChunkingConfig` | ❌ | ❌ | ❌ | ❌ |
| `TokenReductionConfig` | ❌ | ❌ | ❌ | ❌ |
| `LanguageDetectionConfig` | ❌ | ❌ | ❌ | ❌ |
| `ImageExtractionConfig` | ❌ | ❌ | ❌ | ❌ |
| `PostProcessorConfig` | ❌ | ❌ | ❌ | ❌ |

### 3.3 Types (`types.ts`)

| Type | TypeScript Docs | Usage Guide | Examples | Status |
|------|-----------------|-------------|----------|--------|
| `ExtractionResult` | ❌ | ❌ | ❌ | ❌ |
| `ExtractionMetadata` | ❌ | ❌ | ❌ | ❌ |
| `TableData` | ❌ | ❌ | ❌ | ❌ |
| `ChunkData` | ❌ | ❌ | ❌ | ❌ |

### 3.4 TypeScript-Specific Features

| Feature | TypeScript Docs | Usage Guide | Examples | Status |
|---------|-----------------|-------------|----------|--------|
| CLI proxy | ✅ | ✅ | ✅ | ✅ |

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

### 6.1 PDF Extractor (`src/extraction/pdf.rs`)

| Aspect | Rust Docs | Usage Guide | Examples | Status |
|--------|-----------|-------------|----------|--------|
| Module docs | ❌ | ❌ | ❌ | ❌ |
| Public functions | ❌ | ❌ | ❌ | ❌ |

### 6.2 Excel Extractor (`src/extraction/excel.rs`)

| Aspect | Rust Docs | Usage Guide | Examples | Status |
|--------|-----------|-------------|----------|--------|
| Module docs | ❌ | ❌ | ❌ | ❌ |
| Public functions | ❌ | ❌ | ❌ | ❌ |

### 6.3 Email Extractor (`src/extraction/email.rs`)

| Aspect | Rust Docs | Usage Guide | Examples | Status |
|--------|-----------|-------------|----------|--------|
| Module docs | ❌ | ❌ | ❌ | ❌ |
| Public functions | ❌ | ❌ | ❌ | ❌ |

### 6.4 HTML Extractor (`src/extraction/html.rs`)

| Aspect | Rust Docs | Usage Guide | Examples | Status |
|--------|-----------|-------------|----------|--------|
| Module docs | ❌ | ❌ | ❌ | ❌ |
| Public functions | ❌ | ❌ | ❌ | ❌ |

### 6.5 XML Extractor (`src/extraction/xml.rs`)

| Aspect | Rust Docs | Usage Guide | Examples | Status |
|--------|-----------|-------------|----------|--------|
| Module docs | ❌ | ❌ | ❌ | ❌ |
| Public functions | ❌ | ❌ | ❌ | ❌ |

### 6.6 Plain Text Extractor (`src/extraction/text.rs`)

| Aspect | Rust Docs | Usage Guide | Examples | Status |
|--------|-----------|-------------|----------|--------|
| Module docs | ❌ | ❌ | ❌ | ❌ |
| Public functions | ❌ | ❌ | ❌ | ❌ |

### 6.7 Image Extractor (`src/extraction/image.rs`)

| Aspect | Rust Docs | Usage Guide | Examples | Status |
|--------|-----------|-------------|----------|--------|
| Module docs | ❌ | ❌ | ❌ | ❌ |
| Public functions | ❌ | ❌ | ❌ | ❌ |

### 6.8 Structured Data (JSON/YAML/TOML) (`src/extraction/structured.rs`)

| Aspect | Rust Docs | Usage Guide | Examples | Status |
|--------|-----------|-------------|----------|--------|
| Module docs | ❌ | ❌ | ❌ | ❌ |
| Public functions | ❌ | ❌ | ❌ | ❌ |

### 6.9 Pandoc Integration (`src/extraction/pandoc.rs`)

| Aspect | Rust Docs | Usage Guide | Examples | Status |
|--------|-----------|-------------|----------|--------|
| Module docs | ❌ | ❌ | ❌ | ❌ |
| Public functions | ❌ | ❌ | ❌ | ❌ |

### 6.10 LibreOffice Integration (`src/extraction/libreoffice.rs`)

| Aspect | Rust Docs | Usage Guide | Examples | Status |
|--------|-----------|-------------|----------|--------|
| Module docs | ❌ | ❌ | ❌ | ❌ |
| Public functions | ❌ | ❌ | ❌ | ❌ |

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
