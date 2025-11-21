# Kreuzberg Smoke Test Analysis - Comprehensive Report

## Executive Summary

Current Kreuzberg smoke tests are **minimal and insufficient** for comprehensive release validation. They only:
- Test basic import functionality
- Verify extraction from a single text fixture (`report.txt`)
- Assert a simple text pattern match ("smoke")

Missing entirely:
- Java smoke tests (no e2e/smoke/java directory)
- Go smoke tests (no e2e/smoke/go directory)
- Comprehensive functionality coverage across all existing languages
- Multiple file format testing
- Error handling scenarios
- Configuration options testing
- Async/sync variants validation

---

## Part 1: Current Smoke Test Structure

### 1.1 Existing Smoke Tests (4 languages)

#### Python (`e2e/smoke/python/`)
**Files:**
- `main.py` - Single entry point
- `fixtures/report.txt` - Tiny text fixture (96 bytes)
- `requirements-smoke.txt` - kreuzberg>=4.0.0rc1
- `README.md` - Instructions for PyPI/wheel installation

**What it tests:**
```python
result = extract_file_sync(str(fixture))
snippet = result.content.lower()
if "smoke-test" not in snippet:
    raise SystemExit("Smoke test failed: snippet missing")
```

**Coverage:** 
- Import: ✓
- Basic sync extraction: ✓
- Content validation: ✓ (only string presence)
- Async extraction: ✗
- Configuration: ✗
- Multiple formats: ✗
- Error handling: ✗

#### Node.js (`e2e/smoke/node/`)
**Files:**
- `smoke.mjs` - ES module entry point
- `fixtures/report.txt` - Same text fixture (96 bytes)
- `package.json` - Dependencies on kreuzberg-node
- `README.md` - KREUZBERG_NODE_BINDING_PATH environment variable

**What it tests:**
```javascript
const result = extractFileSync(fixture, null, null);
if (!result || typeof result.content !== "string" || !result.content.includes("smoke")) {
    console.error("Smoke test failed: snippet missing");
    process.exit(1);
}
```

**Coverage:**
- Import: ✓
- Basic sync extraction: ✓
- Content validation: ✓ (string presence + type check)
- Async extraction: ✗
- Configuration: ✗
- Multiple formats: ✗
- Error handling: ✗

#### Ruby (`e2e/smoke/ruby/`)
**Files:**
- `app.rb` - Single script
- `fixtures/report.txt` - Same text fixture
- `README.md` - Instructions for gem installation

**What it tests:**
```ruby
result = Kreuzberg.extract_file_sync(fixture)
content = result&.content.to_s
unless content.include?('smoke')
    warn 'Smoke test failed: snippet missing'
    exit 1
end
```

**Coverage:**
- Import: ✓
- Basic sync extraction: ✓
- Content validation: ✓ (string presence)
- Async extraction: ✗
- Configuration: ✗
- Multiple formats: ✗
- Error handling: ✗
- Nil safety: ✓ (safe navigation operator)

#### CLI (`e2e/smoke/cli/`)
**Files:**
- `smoke.sh` - Bash script
- `fixtures/report.txt` - Same text fixture

**What it tests:**
```bash
$BINARY extract "$FIXTURE" --format text > "$OUTPUT"
if ! grep -qi "smoke" "$OUTPUT"; then
    echo "Smoke test failed: snippet missing" >&2
    exit 1
fi
```

**Coverage:**
- CLI availability: ✓
- Basic extraction command: ✓
- Format option: ✓ (text only)
- Content validation: ✓ (grep pattern)
- Multiple commands: ✗
- Configuration flags: ✗
- Error handling: ✗

### 1.2 Missing Language Smoke Tests
- **Java**: No e2e/smoke/java directory → MISSING
- **Go**: No e2e/smoke/go directory → MISSING
- **Rust**: No e2e/smoke/rust directory (core library, maybe OK?)

---

## Part 2: Test Fixtures Analysis

### 2.1 Available Fixture Categories in `/fixtures/`

| Category | Format Examples | Fixture Count |
|----------|-----------------|----------------|
| **pdf** | PDF documents, scanned PDFs | ~10+ fixtures |
| **office** | DOCX, XLSX, PPTX, legacy DOC/XLS/PPT | ~11+ fixtures |
| **image** | PNG, JPG (with/without text) | Multiple fixtures |
| **ocr** | Tesseract-specific PDFs, rotated images | Multiple fixtures |
| **email** | EML format | Multiple fixtures |
| **html** | HTML documents | Multiple fixtures |
| **xml** | XML documents | Multiple fixtures |
| **structured** | JSON, YAML, etc. | Multiple fixtures |
| **plugin_api** | Custom plugin test cases | Multiple fixtures |

### 2.2 Current Smoke Fixture
All 4 existing smoke tests use the SAME fixture:
- **File:** `e2e/smoke/{python,node,ruby,cli}/fixtures/report.txt`
- **Size:** 96 bytes
- **Content:** "This is a small smoke-test document.\nIt verifies that Kreuzberg returns meaningful text output.\n"
- **Format:** Plain text

**Problem:** Only tests text extraction. No coverage of:
- PDF parsing
- Image OCR
- Office document extraction
- Complex structured data
- Error scenarios

---

## Part 3: Public API Coverage Analysis

### 3.1 Rust Core Public API (from `crates/kreuzberg/src/lib.rs`)

**Extraction Functions:**
```rust
pub use core::extractor::{
    // Async variants
    extract_file(...) -> Future<Result<ExtractionResult>>
    extract_bytes(...) -> Future<Result<ExtractionResult>>
    batch_extract_file(...) -> Future<Result<Vec<ExtractionResult>>>
    batch_extract_bytes(...) -> Future<Result<Vec<ExtractionResult>>>
    
    // Sync variants
    extract_file_sync(...) -> Result<ExtractionResult>
    extract_bytes_sync(...) -> Result<ExtractionResult>
    batch_extract_file_sync(...) -> Result<Vec<ExtractionResult>>
    batch_extract_bytes_sync(...) -> Result<Vec<ExtractionResult>>
};
```

**Configuration:**
```rust
pub use core::config::{
    ExtractionConfig,
    OcrConfig,
    PdfConfig,
    ImageExtractionConfig,
    PostProcessorConfig,
    TokenReductionConfig,
    ChunkingConfig,
    EmbeddingConfig,
    EmbeddingModelType,
    LanguageDetectionConfig,
};
```

**MIME Type Handling:**
```rust
pub use core::mime::{
    detect_mime_type(&str) -> &'static str
    detect_mime_type_from_bytes(&[u8]) -> &'static str
    detect_or_validate(...) -> Result<&'static str>
    validate_mime_type(&str) -> Result<&'static str>
    get_extensions_for_mime(&str) -> Vec<&'static str>
};
```

**Plugin System:**
```rust
pub use plugins::registry::{
    get_document_extractor_registry()
    get_ocr_backend_registry()
    get_post_processor_registry()
    get_validator_registry()
};
```

**Other Features:**
```rust
pub use embeddings;  // Feature-gated
pub use chunking;    // Feature-gated
pub use ocr;         // Feature-gated
pub use language_detection; // Feature-gated
pub use keywords;    // Feature-gated (YAKE/RAKE)
pub use stopwords;   // Feature-gated
pub use pdf;         // Feature-gated
pub use utils;       // Quality metrics
```

### 3.2 Python Binding API (thin wrapper)
- `extract_file_sync(path, config=None) -> ExtractionResult`
- `extract_file(path, config=None) -> Awaitable[ExtractionResult]`
- `extract_bytes_sync(data, config=None) -> ExtractionResult`
- `extract_bytes(data, config=None) -> Awaitable[ExtractionResult]`
- Configuration classes: `ExtractionConfig`, `OcrConfig`, `PdfConfig`, etc.
- Plugin registration: `register_post_processor()`, custom validators
- Result types: `ExtractionResult`, `ExtractedTable`, metadata dict

### 3.3 Node.js Binding API (NAPI-RS)
- `extractFileSync(path, config, context) -> ExtractionResult | null`
- `extractFile(path, config, context) -> Promise<ExtractionResult>`
- Configuration builder pattern (TBD from source)
- Result object with `content`, `metadata`, tables

### 3.4 Ruby Binding API (Magnus)
- `Kreuzberg.extract_file_sync(path) -> Result`
- `Kreuzberg.extract_file(path) -> Promise-like`
- Configuration via builder or hash (TBD from source)
- Result with `.content`, `.metadata`

### 3.5 Java Binding API (FFM/Panama)
- `Kreuzberg.extractFileSync(path) -> ExtractionResult`
- `Kreuzberg.extractFile(path) -> CompletableFuture<ExtractionResult>`
- Configuration builder: `new ExtractionConfig.Builder()...build()`
- Result types with getters
- Exception hierarchy: `KreuzbergException`, `ParsingError`, `OcrError`, etc.

### 3.6 Go Binding API (CGO)
- `kreuzberg.ExtractFileSync(path string) (*ExtractionResult, error)`
- `kreuzberg.ExtractFile(ctx context.Context, path string) (*ExtractionResult, error)`
- Configuration structs: `ExtractionConfig`, `OcrConfig`, etc.
- Result struct with `Content`, `Metadata`, `Tables`, etc.

### 3.7 CLI API
Key commands:
- `kreuzberg extract <file> [--format {text|json|csv|...}] [--config <path>]`
- `kreuzberg --version`
- Configuration via flags or config file

---

## Part 4: What Comprehensive Smoke Tests SHOULD Cover

### 4.1 Minimum Test Scenarios Per Language

#### A. Core Functionality
- [x] Import/require/use bindings
- [ ] Extract from file path (sync)
- [ ] Extract from bytes/buffer (sync)
- [ ] Extract async variant
- [ ] Basic configuration usage
- [ ] Result structure validation
- [ ] Multiple file formats (at least PDF + one Office format)

#### B. File Format Coverage
- [ ] **Text** - Plain .txt
- [ ] **PDF** - Native PDF with text
- [ ] **Office** - DOCX (Microsoft Word)
- [ ] **Office** - XLSX (Microsoft Excel)
- [ ] **Office** - PPTX (PowerPoint)
- [ ] **HTML** - Basic HTML document
- [ ] **Image** - PNG/JPG with OCR (if OCR backend available)

#### C. Configuration & Options
- [ ] Default config
- [ ] Custom config (e.g., quality settings, limits)
- [ ] MIME type detection
- [ ] Language detection (if available)
- [ ] Token reduction/quality options
- [ ] Chunking options (if available)

#### D. Error Handling
- [ ] File not found
- [ ] Invalid file path
- [ ] Corrupt/invalid file format
- [ ] Missing dependencies (graceful degradation)
- [ ] Permission errors (if applicable)

#### E. Async/Sync Variants
- [ ] Both async and sync functions work
- [ ] Same output from both paths
- [ ] Concurrent extraction (batch operations)

#### F. Return Value Validation
- [ ] Content is non-empty string
- [ ] Metadata is present and valid
- [ ] Tables extracted (if applicable)
- [ ] MIME type matches input
- [ ] Extraction metadata fields present

#### G. Language-Specific Concerns
- **Python**: Dataclass structure, frozen/hashable, GIL behavior, async context
- **Node.js**: Promise resolution, null handling, Buffer support
- **Ruby**: Safe navigation, symbol/string keys, block syntax
- **Java**: Exception types, resource management, CompletableFuture behavior
- **Go**: Error interface, context cancellation, goroutine safety
- **CLI**: Exit codes, stdout/stderr, format options, JSON output parsing

---

## Part 5: Current Testing Patterns

### 5.1 From E2E Fixtures (Rust tests as reference)

Example from `/e2e/rust/tests/pdf_tests.rs`:

```rust
#[test]
fn test_pdf_assembly_technical() {
    let document_path = resolve_document("pdfs/assembly_language_for_beginners_al4_b_en.pdf");
    if !document_path.exists() {
        println!("Skipping pdf_assembly_technical: missing document at {}", document_path.display());
        return;
    }
    let config = ExtractionConfig::default();
    
    let result = match kreuzberg::extract_file_sync(&document_path, None, &config) {
        Err(err) => panic!("Extraction failed for pdf_assembly_technical: {err:?}"),
        Ok(result) => result,
    };
    
    assertions::assert_expected_mime(&result, &["application/pdf"]);
    assertions::assert_min_content_length(&result, 5000);
    assertions::assert_content_contains_any(&result, &["assembly", "register", "instruction"]);
    assertions::assert_metadata_expectation(&result, "format_type", &serde_json::json!({"eq":"pdf"}));
}
```

**Patterns:**
- Document resolution from fixture path
- Graceful skip if document missing
- Default config usage
- Result validation with multiple assertions
- Content length checks
- Content pattern matching
- Metadata validation

### 5.2 Fixture Schema

From `/fixtures/schema.json`:

```json
{
  "id": "pdf_assembly_technical",
  "description": "Assembly language technical manual...",
  "document": {
    "path": "pdfs/assembly_language_for_beginners_al4_b_en.pdf",
    "media_type": "application/pdf",
    "requires_external_tool": ["pdfium"]
  },
  "extraction": {
    "config": { /* config overrides */ },
    "force_async": false,
    "chunking": { /* chunking expectations */ }
  },
  "assertions": {
    "expected_mime": ["application/pdf"],
    "min_content_length": 5000,
    "max_content_length": null,
    "content_contains_any": ["assembly", "register", "instruction"],
    "content_contains_all": null,
    "metadata_expectations": { ... }
  }
}
```

---

## Part 6: Smoke Test Gaps & Recommendations

### 6.1 Critical Gaps

| Gap | Current | Should Be | Impact |
|-----|---------|-----------|--------|
| **Number of languages tested** | 4 (Python, Node, Ruby, CLI) | 6 (add Java, Go) | Release validation incomplete |
| **File formats** | 1 text file | 5-7 formats | Can't detect extraction failures for major formats |
| **Configuration testing** | None | At least 1 config variant | Can't catch config binding bugs |
| **Error handling** | None | Invalid file, missing deps | Exception hierarchy untested |
| **Async variants** | Python only (implicit), Ruby/Node unclear | Explicit async+sync | Async bugs invisible to smoke tests |
| **Batch operations** | None | 1 test with 2+ files | Batch mode untested |
| **Return value validation** | Basic (string presence) | Structure, types, fields | Silent failures possible |

### 6.2 Why Current Tests Are Insufficient

1. **Single fixture across all tests** → Can't validate multiple format support
2. **Text-only fixture** → No PDF, image, or office format validation
3. **Only success path tested** → Error handling invisible
4. **Minimal assertions** → Only checks for text presence, not structure
5. **No performance baselines** → Can't catch regressions
6. **No async testing** (except Python) → Async/await bugs hidden
7. **No configuration testing** → Config binding bugs not caught
8. **Missing languages** → Java/Go users can't verify installation

---

## Part 7: Fixture Strategy for Comprehensive Smoke Tests

### 7.1 Recommended Smoke Test Fixtures

Instead of distributing large files (PDFs, images), smoke tests should use:
- **Small text fixtures** (< 1KB each)
- **Minimal binary fixtures** (< 10KB each)
- **Embedded or downloaded on demand**

#### Proposed Fixtures:
1. **Text** - `smoke.txt` (96 bytes) ✓ Exists
2. **PDF** - `smoke.pdf` (< 50KB, single page)
3. **DOCX** - `smoke.docx` (< 100KB, minimal)
4. **HTML** - `smoke.html` (< 1KB)
5. **Image** - `smoke.png` (< 100KB, with clear text)

### 7.2 Where to Store Fixtures

Option A: **Embed base64 in test code** (simplest for smoke tests)
```python
SMOKE_PDF_BASE64 = "JVBERi0xLjMK..."
with tempfile.NamedTemporaryFile(suffix=".pdf", delete=False) as f:
    f.write(base64.b64decode(SMOKE_PDF_BASE64))
    result = extract_file_sync(f.name)
```

Option B: **Download from GitHub Releases** (cleaner, larger files)
```python
FIXTURE_URL = "https://github.com/.../releases/download/fixtures-v1/smoke-pdf.pdf"
# Download if needed, cache locally
```

Option C: **Symlink to fixtures/ directory** (current approach)
- ✓ Works for E2E tests with full repo
- ✗ Doesn't work for PyPI wheel tests
- ✗ Doesn't work for standalone binary distributions

---

## Part 8: Reference: html-to-markdown Smoke Tests (if available)

The analysis mentions "html-to-markdown as a reference" but I cannot access external repositories. However, best practices for smoke tests in modern projects typically include:

1. **Multi-language verification**: Test each language binding
2. **Format coverage**: Sample of supported formats
3. **Configuration variants**: At least 1 non-default config
4. **Error paths**: At least 1 error scenario
5. **Async/sync both**: Ensure both work
6. **Quick execution**: < 30 seconds total
7. **Clear output**: PASS/FAIL with error details
8. **CI integration**: Separate from long E2E tests
9. **Minimal dependencies**: Avoid external services
10. **Reproducibility**: No flaky tests, deterministic results

---

## Part 9: Summary & Action Items

### What Works Well
- ✓ Simple, fast execution (< 1 second per test)
- ✓ Clear error messages
- ✓ No external dependencies
- ✓ Easy to run locally
- ✓ CI-friendly structure

### What Needs Improvement
- ✗ Coverage: Only 4 languages (missing Java, Go)
- ✗ Formats: Only text (no PDF, images, Office)
- ✗ Functionality: Only sync extraction (no async, batch, config)
- ✗ Errors: No error path testing
- ✗ Validation: Only string presence, not structure

### Recommended Enhancements (Priority Order)

1. **CRITICAL**: Add Java and Go smoke tests
   - Create `e2e/smoke/java/` with Maven structure
   - Create `e2e/smoke/go/` with Go module
   
2. **HIGH**: Add format coverage
   - Add PDF fixture to all languages
   - Add DOCX fixture to all languages
   - Test extraction with realistic content validation
   
3. **HIGH**: Add configuration testing
   - Each language tests with custom `ExtractionConfig`
   - Validate config is applied
   
4. **MEDIUM**: Add error handling
   - Test file not found
   - Test invalid format
   - Verify exception types
   
5. **MEDIUM**: Add async testing
   - Node/Ruby/Java: Add async variant
   - Validate async + sync produce same results
   
6. **LOW**: Performance baselines
   - Track extraction time in smoke tests
   - Alert on significant regressions

