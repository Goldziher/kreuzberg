# Kreuzberg Smoke Tests: Coverage Matrix

## Feature Coverage by Language

```
                    Python   Node    Ruby    CLI    Java    Go
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
BASIC FUNCTIONALITY
Import/Load          ✓        ✓       ✓       ✓      ✗      ✗
File Extraction      ✓        ✓       ✓       ✓      ✗      ✗
Bytes Extraction     ✗        ?       ?       ✗      ✗      ✗
Result Structure     △        △       △       ✓      ✗      ✗
Metadata Access      ?        ?       ?       ✓      ✗      ✗

FUNCTIONALITY VARIANTS
Sync API             ✓        ✓       ✓       ✓      ✗      ✗
Async API            ✓        ✗       ✗       ✗      ✗      ✗
Batch Operations     ✗        ✗       ✗       ✗      ✗      ✗
Concurrent Ext.      ✗        ✗       ✗       ✗      ✗      ✗

FILE FORMAT SUPPORT
Text (.txt)          ✓        ✓       ✓       ✓      ✗      ✗
PDF                  ✗        ✗       ✗       ✗      ✗      ✗
DOCX                 ✗        ✗       ✗       ✗      ✗      ✗
XLSX                 ✗        ✗       ✗       ✗      ✗      ✗
PPTX                 ✗        ✗       ✗       ✗      ✗      ✗
HTML                 ✗        ✗       ✗       ✗      ✗      ✗
Images               ✗        ✗       ✗       ✗      ✗      ✗
Email                ✗        ✗       ✗       ✗      ✗      ✗

CONFIGURATION
Default Config       ✓        ✓       ✓       ✓      ✗      ✗
Custom Config        ✗        ✗       ✗       ✗      ✗      ✗
MIME Type Options    ✗        ✗       ✗       ✗      ✗      ✗
OCR Config           ✗        ✗       ✗       ✗      ✗      ✗
Quality Settings     ✗        ✗       ✗       ✗      ✗      ✗

ERROR HANDLING
File Not Found       ✗        ✗       ✗       ✗      ✗      ✗
Invalid File         ✗        ✗       ✗       ✗      ✗      ✗
Corrupt File         ✗        ✗       ✗       ✗      ✗      ✗
Missing Dependencies ✗        ✗       ✗       ✗      ✗      ✗
Permission Errors    ✗        ✗       ✗       ✗      ✗      ✗

LANGUAGE-SPECIFIC
Exception Hierarchy  ✗        ✗       ✗       ✗      ✗      ✗
Type Safety          ?        ?       ?       ✓      ✗      ✗
Resource Cleanup     ?        ?       ?       ✓      ✗      ✗
Context/Timeout      ✗        ✗       ✗       ✗      ✗      ✗

ASSERTIONS
Content Length       △        △       △       ✓      ✗      ✗
Content Pattern      ✓        ✓       ✓       ✓      ✗      ✗
Metadata Fields      ✗        ✗       ✗       ✗      ✗      ✗
MIME Type Correct    ?        ?       ?       ✓      ✗      ✗
Table Extraction     ✗        ✗       ✗       ✗      ✗      ✗

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
✓ = Tested
△ = Partially tested / Not comprehensive
? = Unknown / Unclear
✗ = Not tested
```

## Detailed Coverage Analysis

### Python
- **Strengths:**
  - Async extraction tested
  - Clean error messages
  - Uses dataclass result types
- **Weaknesses:**
  - Only text fixture
  - No configuration testing
  - No error path testing
  - No multi-format validation
  - Minimal result structure validation

### Node.js
- **Strengths:**
  - Null safety check
  - Type validation (string check)
  - ES module syntax
- **Weaknesses:**
  - Only text fixture
  - No async variant tested
  - No configuration
  - No error handling
  - Missing binding path flexibility

### Ruby
- **Strengths:**
  - Safe navigation operator usage
  - Clean syntax
  - Gem installation tested
- **Weaknesses:**
  - Only text fixture
  - No async variant
  - No configuration
  - No error handling
  - Minimal type checking

### CLI
- **Strengths:**
  - Format option tested
  - Exit codes validated
  - Real binary execution
  - Output format parsing
- **Weaknesses:**
  - Only text fixture
  - No advanced options
  - Limited error scenarios
  - Single command tested

### Java
- **Current Status:** NO SMOKE TEST EXISTS
- **Critical for:** FFM API validation, exception types, resource management
- **Would test:** CompletableFuture async, builder pattern, exception hierarchy

### Go
- **Current Status:** NO SMOKE TEST EXISTS
- **Critical for:** CGO binding validation, error interface, context handling
- **Would test:** Goroutine safety, context cancellation, error wrapping

---

## File Format Gap Analysis

### Supported Formats (from Rust core)
- Text files
- PDF documents
- Office: DOCX, XLSX, PPTX, DOC, XLS, PPT
- HTML
- XML
- Images (PNG, JPG, WEBP, etc.)
- Email (EML)
- Archives (ZIP, TAR, etc.)
- Email with attachments
- Structured data (JSON, YAML, etc.)

### Smoke Test Coverage
- ✓ Text: 1 fixture (report.txt)
- ✗ PDF: 0 fixtures
- ✗ Office: 0 fixtures
- ✗ HTML: 0 fixtures
- ✗ XML: 0 fixtures
- ✗ Images: 0 fixtures
- ✗ Email: 0 fixtures
- ✗ Structured: 0 fixtures

**Coverage:** 1/9 format categories (11%)

---

## Configuration Options Gap Analysis

### Supported Configuration Options
From `crates/kreuzberg/src/core/config.rs`:

1. **OcrConfig**
   - backend selection
   - confidence thresholds
   - preprocessing options

2. **PdfConfig**
   - text extraction mode
   - image rendering
   - metadata extraction

3. **ImageExtractionConfig**
   - DPI settings
   - preprocessing
   - resize options

4. **PostProcessorConfig**
   - quality filters
   - token reduction
   - text normalization

5. **TokenReductionConfig**
   - reduction strategy
   - preserve rate
   - semantic filtering

6. **ChunkingConfig**
   - chunk size
   - overlap
   - splitter strategy

7. **EmbeddingConfig**
   - model selection
   - batch processing
   - device selection

8. **LanguageDetectionConfig**
   - confidence threshold
   - fallback language

### Smoke Test Coverage
- ✓ Default config: All languages
- ✗ Custom config: No language
- ✗ OCR config: No language
- ✗ PDF config: No language
- ✗ Quality config: No language
- ✗ Chunking config: No language

**Coverage:** 1/8 config categories (12%)

---

## Fixture Distribution Analysis

### Test Fixture Locations
```
e2e/smoke/python/fixtures/report.txt      (96 bytes, text)
e2e/smoke/node/fixtures/report.txt        (96 bytes, text, symlink?)
e2e/smoke/ruby/fixtures/report.txt        (96 bytes, text, symlink?)
e2e/smoke/cli/fixtures/report.txt         (96 bytes, text, symlink?)
```

### Problem: Symlink Dependencies
If using symlinks, smoke tests will fail when:
1. Testing with installed packages (PyPI wheel, npm, RubyGems)
2. Testing without full repo (CI/CD minimal builds)
3. Distributing only the binding, not full source

### Solution Options

**Option 1: Embed fixtures as base64**
- Pros: Self-contained, works everywhere
- Cons: Binary bloat
- Files: Single Python/JS/etc file with encoded data

**Option 2: Download on first run**
- Pros: Clean, cacheable
- Cons: Network dependency, slower first run
- Files: Download script + cache directory

**Option 3: Include minimal fixtures**
- Pros: Small size, simple
- Cons: Limited coverage
- Files: Keep current text fixtures + add 1-2 small binaries

**Option 4: Separate fixture distribution**
- Pros: Flexible, scalable
- Cons: Additional package to maintain
- Files: separate wheels/packages for fixtures

---

## Async/Sync Testing Gaps

### API Availability by Language

| Language | Sync | Async | Tested |
|----------|------|-------|--------|
| Python   | ✓    | ✓     | Sync ✓, Async ✓ |
| Node.js  | ✓    | ✓     | Sync ✓, Async ✗ |
| Ruby     | ✓    | ?     | Sync ✓, Async ? |
| Java     | ✓    | ✓ (Future) | None ✗ |
| Go       | ✓    | ✓ (goroutine) | None ✗ |
| CLI      | ✓    | ✗     | Sync ✓ |

### Critical Tests Missing
1. Node async/await with Promise
2. Ruby async (if supported)
3. Java CompletableFuture behavior
4. Go goroutine safety
5. Result equivalence: async vs sync

---

## Error Path Testing

### Error Types Should Be Tested

```
File Not Found
├── Missing file path → FileNotFoundError
├── Invalid file type → ParsingError
├── Corrupt file → ParsingError
└── Directory instead of file → InvalidFileError

Missing Dependencies
├── OCR backend not installed → MissingDependencyError
├── Pandoc not available → MissingDependencyError
└── LibreOffice not found → MissingDependencyError

Permission Errors
├── Read permission denied → PermissionError
└── Directory traversal → SecurityError

Extraction Errors
├── Unsupported format → UnsupportedFormatError
├── Timeout during extraction → TimeoutError
└── Memory exceeded → OutOfMemoryError
```

### Current Coverage
- None tested in smoke tests
- All error paths invisible to release validation

---

## Recommendations Summary

### Phase 1: Critical (Missing Languages)
1. Add Java smoke tests (~1 day)
   - Basic extraction + exception types
   - CompletableFuture async
   - Builder pattern validation
   
2. Add Go smoke tests (~1 day)
   - Basic extraction + error interface
   - Goroutine-safe concurrent extraction
   - Context cancellation

### Phase 2: High Priority (Formats & Config)
1. Add PDF fixture to all languages
2. Add DOCX fixture to all languages
3. Add custom config test to all languages
4. Test MIME type detection

### Phase 3: Medium Priority (Async & Errors)
1. Test async variants in Node/Ruby/Java
2. Add 2-3 error path tests per language
3. Batch operation tests

### Phase 4: Polish (Advanced)
1. Performance baselines
2. Plugin system (Python custom processors)
3. Advanced config options
4. Concurrent extraction patterns

---

## Estimated Implementation Time

| Task | Time | Language(s) |
|------|------|-------------|
| Java smoke test | 2 days | Java |
| Go smoke test | 2 days | Go |
| PDF fixtures (all) | 1 day | All |
| DOCX fixtures (all) | 1 day | All |
| Configuration testing | 1 day | All |
| Async variants | 2 days | Node, Ruby, Java |
| Error handling | 2 days | All |
| **Total** | **11 days** | |

---

## Risk Assessment: Current Smoke Tests

### What Could Break Silently?
1. ✓ PDF extraction entirely broken (not tested)
2. ✓ Office document extraction broken (not tested)
3. ✓ Async/await deadlock in Python (might work but untested async in other langs)
4. ✓ Configuration not applied (not tested)
5. ✓ Exception hierarchy broken (not tested)
6. ✓ Type safety violations (minimal validation)
7. ✓ Memory leaks in long-running processes (not tested)
8. ✓ Plugin system broken (not tested)
9. ✗ Basic import broken (tested)
10. ✗ Text extraction entirely broken (tested)

### Release Confidence: ~20%
- Only text extraction validated
- Only 4 of 6 language bindings tested
- Only 1 of ~50 extraction formats tested
- No configuration testing
- No error path testing
- No async testing (except Python implicitly)

