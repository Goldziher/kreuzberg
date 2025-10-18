# Kreuzberg V4 - Remaining Tasks

**Status**: Testing & Integration Phase
**Last Updated**: 2025-10-17
**Test Status**: 882 tests passing ✅ (854 core + 7 API + 18 integration + 3 MCP)
**Coverage**: ~85% (target: 95%)

______________________________________________________________________

## 🎯 HIGH PRIORITY: Comprehensive Integration Testing

### TEST-SUITE-1: File Format Integration Tests (6-8 hours)

**Priority**: P0 - Critical for production readiness
**Test Files**: 178 real documents in `test_documents/` directory

#### Overview

Test all supported file formats end-to-end with real documents from the test suite. These tests verify:

- Extraction produces non-empty, sensible content
- MIME type detection works correctly
- Metadata fields are populated appropriately for each format
- No crashes or panics on real-world files

#### File: `tests/format_integration.rs` (NEW)

**Structure**:

```rust
mod pdf_tests;        // 44 PDFs available
mod office_tests;     // 16 Word + 6 Excel + 6 PowerPoint
mod image_tests;      // 13 images (OCR required)
mod web_tests;        // 20 HTML files
mod text_tests;       // 23 text/markdown files
mod data_tests;       // JSON, YAML, TOML, XML
mod email_tests;      // 11 email files
mod archive_tests;    // ZIP, TAR, 7Z files
```

**Test Categories**:

1. **PDF Tests** (20 tests - 2 hours)

    - ✅ **Sample files**:
        - `pdfs/simple_text.pdf` - Basic text extraction
        - `pdfs/code_and_formula.pdf` - Math formulas
        - `pdfs/copy_protected.pdf` - Password-protected (should fail gracefully)
        - `pdfs/a_course_in_machine_learning_ciml_v0_9_all.pdf` - Large file (447 pages)
    - **Tests**:
        - `test_pdf_simple_text_extraction()` - Basic PDF
        - `test_pdf_with_images()` - PDF with embedded images
        - `test_pdf_with_tables()` - Table extraction
        - `test_pdf_scanned_with_ocr()` - Scanned PDF requiring OCR
        - `test_pdf_large_document()` - 400+ page PDF
        - `test_pdf_password_protected_fail()` - Should return error
        - `test_pdf_metadata_extraction()` - Title, author, dates
        - `test_pdf_multi_language()` - Non-English PDFs
        - **Avoid**: Testing pdfium rendering internals (that's library testing)

1. **Office Document Tests** (15 tests - 1.5 hours)

    - ✅ **Sample files**:
        - `office/document.docx` - Word document
        - `office/excel.xlsx` - Excel spreadsheet
        - `presentations/*.pptx` - PowerPoint files
        - `legacy_office/*.doc` - Legacy Office (LibreOffice conversion)
    - **Tests**:
        - `test_docx_basic_extraction()` - Word document
        - `test_docx_with_tables()` - Tables in Word
        - `test_docx_with_images()` - Images in Word
        - `test_xlsx_sheet_extraction()` - Excel data
        - `test_xlsx_multiple_sheets()` - Multi-sheet workbook
        - `test_xlsx_formulas()` - Formula evaluation
        - `test_pptx_slide_extraction()` - PowerPoint slides
        - `test_pptx_with_notes()` - Speaker notes
        - `test_legacy_doc_conversion()` - .doc files (LibreOffice)
        - `test_legacy_ppt_conversion()` - .ppt files
        - **Avoid**: Testing python-pptx or calamine internals

1. **Image + OCR Tests** (12 tests - 2 hours)

    - ✅ **Sample files**:
        - `images/test_hello_world.png` - Simple English text
        - `images/english_and_korean.png` - Multi-language
        - `images/chi_sim_image.jpeg` - Chinese text
        - `images/jpn_vert.jpeg` - Japanese vertical text
        - `images/invoice_image.png` - Invoice OCR
        - `images/flower_no_text.jpg` - No text (should return empty)
    - **Tests**:
        - `test_ocr_simple_english()` - Basic OCR
        - `test_ocr_multi_language()` - Korean + English
        - `test_ocr_chinese_text()` - Chinese characters
        - `test_ocr_japanese_vertical()` - Vertical Japanese
        - `test_ocr_invoice_layout()` - Complex layout
        - `test_ocr_no_text_image()` - Image without text
        - `test_ocr_with_table_detection()` - Table in image
        - `test_ocr_language_detection()` - Auto language detection
        - `test_ocr_caching()` - Verify OCR cache works
        - **Good**: Test OCR integration, not Tesseract accuracy
        - **Avoid**: Testing specific OCR accuracy percentages (flaky)

1. **HTML/Web Tests** (8 tests - 1 hour)

    - ✅ **Sample files**:
        - `web/simple_table.html` - HTML with tables
        - `web/taylor_swift.html` - Wikipedia article
        - `web/germany_german.html` - Non-English
    - **Tests**:
        - `test_html_to_markdown()` - HTML conversion
        - `test_html_table_extraction()` - Preserve tables
        - `test_html_non_english()` - UTF-8 handling
        - `test_html_complex_layout()` - Nested elements
        - **Avoid**: Testing html-to-markdown library internals

1. **Text/Markdown Tests** (6 tests - 45 min)

    - ✅ **Sample files**:
        - `text/*.md` - Markdown files
        - `text/*.txt` - Plain text
    - **Tests**:
        - `test_markdown_metadata_extraction()` - Headers, links, code blocks
        - `test_plain_text_streaming()` - Large text file
        - `test_text_encoding_detection()` - UTF-8, Latin-1
        - **Good**: Test metadata extraction correctness
        - **Avoid**: Testing every Markdown edge case (that's unit test territory)

1. **Data Format Tests** (8 tests - 1 hour)

    - ✅ **Sample files**:
        - `data_formats/simple.json` - JSON
        - `data_formats/simple.yaml` - YAML
        - `xml/*.xml` - XML files
    - **Tests**:
        - `test_json_extraction()` - JSON parsing
        - `test_json_nested_structure()` - Deep nesting
        - `test_yaml_extraction()` - YAML parsing
        - `test_xml_extraction()` - XML element extraction
        - `test_xml_large_file()` - Streaming parser
        - **Good**: Test extraction works, structure preserved
        - **Avoid**: Testing serde_json/serde_yaml internals

1. **Email Tests** (6 tests - 45 min)

    - ✅ **Sample files**: `email/*.eml`
    - **Tests**:
        - `test_email_basic_extraction()` - Subject, body
        - `test_email_with_attachments()` - Attachment list
        - `test_email_metadata()` - From, To, Cc, Date
        - `test_email_multipart()` - HTML + text parts

1. **Archive Tests** (5 tests - 45 min)

    - Tests for ZIP, TAR, 7Z extraction
    - **Tests**:
        - `test_zip_extraction()` - Extract files from ZIP
        - `test_tar_extraction()` - TAR archive
        - `test_nested_archives()` - Archive in archive

**Test Helpers** (`tests/helpers/mod.rs`):

```rust
// Shared test utilities
fn assert_non_empty_content(result: &ExtractionResult)
fn assert_metadata_field_exists(result: &ExtractionResult, field: &str)
fn load_test_file(relative_path: &str) -> Vec<u8>
fn get_test_documents_dir() -> PathBuf
```

**Success Criteria**:

- All 60+ tests pass
- No panics or crashes on any test file
- Coverage increases to 90%+
- Metadata fields populated for each format

______________________________________________________________________

### TEST-SUITE-2: API Integration Tests (3-4 hours)

**Priority**: P0 - Critical (API has only 7 basic tests)
**Goal**: Comprehensive Axum testing with real documents

#### File: `tests/api_integration.rs` (EXPAND EXISTING)

**Current**: 7 tests (health, info, extract basic)
**Target**: 30+ tests

**Test Categories**:

1. **Endpoint Tests** (8 tests - 1 hour)

    - `test_health_endpoint()` ✅ (exists)
    - `test_info_endpoint()` ✅ (exists)
    - `test_cache_stats_endpoint()` - GET /cache/stats
    - `test_cache_clear_endpoint()` - DELETE /cache/clear
    - `test_extract_endpoint_404()` - Invalid route
    - `test_cors_headers()` - CORS configuration
    - `test_options_request()` - Preflight handling

1. **Extract Endpoint Tests** (12 tests - 1.5 hours)

    - `test_extract_no_files()` ✅ (exists)
    - `test_extract_text_file()` ✅ (exists)
    - `test_extract_multiple_files()` ✅ (exists)
    - `test_extract_with_config()` ✅ (exists)
    - `test_extract_invalid_config()` ✅ (exists)
    - `test_extract_pdf_file()` - Upload PDF
    - `test_extract_docx_file()` - Upload Word
    - `test_extract_xlsx_file()` - Upload Excel
    - `test_extract_large_file()` - 10MB+ file
    - `test_extract_binary_file()` - Binary data
    - `test_extract_with_mime_override()` - Force MIME type
    - `test_extract_unsupported_format()` - Should return error
    - `test_extract_empty_file()` - 0 byte file
    - `test_extract_concurrent_requests()` - 10 parallel requests

1. **Error Handling Tests** (6 tests - 1 hour)

    - `test_extract_malformed_multipart()` - Bad request body
    - `test_extract_missing_content_type()` - No content-type header
    - `test_extract_oversized_file()` - Exceeds limit (if configured)
    - `test_extract_invalid_utf8()` - Bad text encoding
    - `test_extract_corrupted_pdf()` - Malformed file
    - `test_api_error_format()` - Error response structure

1. **Configuration Tests** (4 tests - 30 min)

    - `test_server_default_config()` - Uses discovered config
    - `test_per_request_config_override()` - Override OCR settings
    - `test_invalid_config_override()` - Bad JSON config
    - `test_config_validation()` - Invalid settings rejected

**Test Helpers**:

```rust
fn create_test_multipart_request(files: Vec<(&str, Vec<u8>)>) -> Request<Body>
fn assert_extraction_response(response: Response, expected_count: usize)
fn load_pdf_bytes() -> Vec<u8>
```

**Success Criteria**:

- 30+ comprehensive API tests
- All error paths tested
- Multipart upload handling validated
- Concurrent request handling verified

______________________________________________________________________

### TEST-SUITE-3: OCR Integration Tests (2-3 hours)

**Priority**: P1 - Important for OCR workflows
**Goal**: End-to-end OCR testing with real images

#### File: `tests/ocr_integration.rs` (NEW)

**Test Categories**:

1. **Tesseract Integration** (8 tests - 1.5 hours)

    - `test_tesseract_basic_ocr()` - Simple English
    - `test_tesseract_language_support()` - eng, deu, fra, jpn, chi_sim
    - `test_tesseract_psm_modes()` - Different page segmentation modes
    - `test_tesseract_hocr_output()` - hOCR format
    - `test_tesseract_pdf_output()` - Searchable PDF
    - `test_tesseract_confidence_scores()` - Metadata includes confidence
    - **Good**: Test Tesseract integration works
    - **Avoid**: Testing Tesseract accuracy (that's upstream's job)

1. **OCR Caching** (4 tests - 30 min)

    - `test_ocr_cache_hit()` - Second extraction uses cache
    - `test_ocr_cache_miss()` - Cache invalidation
    - `test_ocr_cache_disabled()` - Bypass cache
    - `test_ocr_cache_stats()` - Cache statistics

1. **PDF + OCR** (6 tests - 1 hour)

    - `test_pdf_force_ocr()` - Force OCR on text PDF
    - `test_pdf_scanned_auto_ocr()` - Detect scan, apply OCR
    - `test_pdf_mixed_content()` - Text + scanned pages
    - `test_pdf_ocr_large_file()` - Multi-page scan

**Success Criteria**:

- OCR integration verified
- Caching behavior validated
- Language support confirmed
- No memory leaks on large OCR jobs

______________________________________________________________________

### TEST-SUITE-4: Error & Edge Case Tests (1-2 hours)

**Priority**: P2 - Important for robustness
**File**: `tests/error_handling.rs` (NEW)

**Test Categories**:

1. **Corrupted Files** (6 tests)

    - `test_truncated_pdf()` - Incomplete PDF
    - `test_corrupted_zip()` - Bad archive
    - `test_invalid_xml()` - Malformed XML
    - `test_corrupted_image()` - Bad image data
    - **Good**: Verify graceful error handling
    - **Avoid**: Testing every possible corruption (impractical)

1. **Edge Cases** (8 tests)

    - `test_empty_file()` - 0 bytes
    - `test_very_large_file()` - 100MB+ file
    - `test_deeply_nested_json()` - 1000+ levels
    - `test_unicode_filename()` - Non-ASCII names
    - `test_special_characters()` - Emojis, RTL text
    - `test_concurrent_extraction_stress()` - 100 parallel extractions

1. **Security Tests** (4 tests)

    - `test_path_traversal_prevention()` - ../../../etc/passwd
    - `test_zip_bomb_protection()` - Compression bomb (if implemented)
    - `test_xml_billion_laughs()` - XML entity expansion
    - **Good**: Verify security isn't broken
    - **Avoid**: Deep security audit (that's a separate process)

______________________________________________________________________

## 🏗️ Architecture Refactoring

### HIGH-5: Strongly-Typed Metadata Architecture (COMPLETED SEPARATELY)

**Status**: Scheduled but moved to separate issue
**Reason**: Large architectural change, deserves dedicated focus
**Timeline**: After integration tests are complete

______________________________________________________________________

## 📦 Optional Enhancements

### FEATURE-4: Zero-Copy Bytes Support (1.5 hours)

**Priority**: P3 - Performance optimization (internal only)
**File**: `crates/kreuzberg-py/src/core.rs`

**Tasks**:

1. Use buffer protocol for zero-copy bytes
1. Benchmark improvement
1. **Note**: Internal optimization, doesn't affect CLI/API/MCP users

______________________________________________________________________

## 📊 Progress Summary

### Completed ✅

- High priority refactoring (4/4 tasks)
- Config file support (CLI, API, MCP)
- Cache management (CLI, API, MCP)
- All clippy errors fixed
- PyO3 dead code removed
- Pdfium download optimized

### In Progress 🚧

- Integration test suite design ← **YOU ARE HERE**

### Time Estimates

- **TEST-SUITE-1**: Format integration - 6-8 hours
- **TEST-SUITE-2**: API integration - 3-4 hours
- **TEST-SUITE-3**: OCR integration - 2-3 hours
- **TEST-SUITE-4**: Error/edge cases - 1-2 hours
- **Total**: 12-17 hours for comprehensive testing

### Success Criteria

- ✅ No critical issues
- ✅ No memory leaks
- ✅ Error context preserved
- ✅ Single source of truth
- ✅ GIL management documented
- ✅ Cache + config in all interfaces
- 🎯 **95%+ test coverage** (currently ~85%, target with new tests: 95%+)
- 🎯 **All supported formats tested** (60+ format tests)
- 🎯 **API fully tested** (30+ API tests)
- 🎯 **OCR integration validated** (18+ OCR tests)

### Recommended Approach

**Phase 1** (4-5 hours): Core format tests

- Start with `tests/format_integration.rs`
- Implement PDF, Office, Image test modules
- Get immediate coverage boost

**Phase 2** (3-4 hours): API tests

- Expand `tests/api_integration.rs`
- Test all endpoints thoroughly
- Validate error handling

**Phase 3** (2-3 hours): OCR tests

- Create `tests/ocr_integration.rs`
- Test Tesseract integration
- Validate caching

**Phase 4** (1-2 hours): Edge cases

- Create `tests/error_handling.rs`
- Test corrupted files, edge cases
- Stress testing

**Parallel Option**: Run test suites in parallel for faster iteration

______________________________________________________________________

## 💡 Testing Philosophy

### ✅ Good Integration Tests

- Test real-world scenarios end-to-end
- Use actual files from test suite
- Verify behavior, not implementation
- Test error conditions gracefully
- Test concurrent operations
- Test with real data at boundaries (large files, many files)

### ❌ Bad Integration Tests (Avoid)

- Testing library internals (pdfium, tesseract accuracy)
- Over-mocking (defeats purpose of integration tests)
- Testing every edge case (that's unit tests)
- Flaky tests (OCR accuracy percentages)
- Redundant tests that add no value
- Testing private implementation details

______________________________________________________________________

## 📝 Notes

- Test files available in `/test_documents/` (178 real documents)
- Current coverage: ~85% (mostly unit tests)
- Target coverage: 95%+ (with integration tests)
- Focus on **behavior** not **implementation**
- Integration tests complement unit tests, don't replace them
