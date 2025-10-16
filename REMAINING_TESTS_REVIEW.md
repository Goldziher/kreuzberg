# Remaining Test Files Review

After migration, 60 test files remain in `./tests` that test private/internal functionality.

## Summary

- **Total**: 60 files
- **Recommendation**: DELETE ALL - none test public API behavior
- **Reason**: All test internal implementation details (extractor classes, utility functions, OCR backends) that are now handled by Rust

---

## Category Breakdown

### 1. EXTRACTORS (26 files) - DELETE ALL

These test internal extractor classes (`PDFExtractor`, `EmailExtractor`, etc.) and their private methods:

**Files testing internal extractor methods:**
- `base_extractor_test.py` - Tests `Extractor` base class (internal)
- `base_memory_limits_test.py` - Tests `_check_image_memory_limits()` (internal method)
- `base_ocr_processing_test.py` - Tests `_process_images_with_ocr()` (internal method)
- `base_ocr_simple_test.py` - Tests OCR image filtering (internal logic)
- `pdf_test.py` - Tests `PDFExtractor._extract_pdf_searchable_text()` (internal)
- `email_test.py` - Tests `EmailExtractor.extract_bytes_sync()` (internal)
- `html_test.py` - Tests `HTMLExtractor` (internal)
- `image_test.py` - Tests `ImageExtractor` (internal)
- `json_test.py` - Tests `JSONExtractor` (internal)
- `xml_test.py` - Tests `XMLExtractor` (internal)
- `text_test.py` - Tests `PlainTextExtractor` (internal)
- `spreadsheet_test.py` - Tests `SpreadsheetExtractor` (internal)
- `presentation_test.py` - Tests `PresentationExtractor` (internal)
- `pandoc_test.py` - Tests `PandocExtractor` (internal)
- `legacy_office_test.py` - Tests legacy Office format extraction (internal)

**Files testing internal error handling:**
- `email_error_paths_test.py` - Tests `EmailExtractor` error paths
- `image_error_handling_test.py` - Tests `ImageExtractor` error handling
- `image_error_simple_test.py` - Tests `ImageExtractor` simple errors
- `html_invalid_base64_test.py` - Tests HTML base64 handling

**Files testing internal features:**
- `pdf_images_test.py` - Tests `PDFExtractor._extract_images()` (internal)
- `pdf_sync_images_test.py` - Tests sync image extraction (internal)
- `image_deduplication_test.py` - Tests image dedup logic (internal)
- `pandoc_metadata_test.py` - Tests Pandoc metadata extraction (internal)
- `presentation_compatibility_test.py` - Tests PPTX compatibility (internal)
- `structured_test.py` - Tests structured data extraction (internal)
- `email_real_files_test.py` - Tests email extraction with real files (internal)

**Why DELETE**: All extraction is now handled by Rust. Public API is `extract_file()`, `extract_bytes()` - which ARE tested in migrated tests.

---

### 2. OCR (11 files) - DELETE ALL

These test internal OCR backend implementations:

- `base_test.py` - Tests `OCRBackend` base class (internal)
- `tesseract_test.py` - Tests `TesseractBackend` (internal)
- `easyocr_test.py` - Tests `EasyOCRBackend` (internal)
- `paddleocr_test.py` - Tests `PaddleBackend` (internal)
- `tesseract_behavior_test.py` - Tests Tesseract internal behavior
- `tesseract_edge_cases_test.py` - Tests Tesseract edge cases (internal)
- `tesseract_normalization_test.py` - Tests text normalization (internal)
- `tesseract_tsv_test.py` - Tests TSV output parsing (internal)
- `tesseract_benchmark_test.py` - Benchmarks Tesseract (internal)
- `quality_regression_test.py` - Tests OCR quality regression (internal)
- `init_test.py` - Tests OCR module initialization (internal)

**Why DELETE**: OCR is now handled by Rust's Tesseract integration. Public API is via `ExtractionConfig(ocr=OcrConfig(...))` - which IS tested in migrated tests.

---

### 3. UTILS (15 files) - DELETE ALL

These test internal utility functions:

**Rust-migrated utilities:**
- `quality_test.py` - Tests `calculate_quality_score()` (now in Rust `_internal_bindings`)
- `string_test.py` - Tests string utilities (now in Rust)
- `table_test.py` - Tests table utilities (now in Rust)
- `serialization_test.py` - Tests msgspec serialization helpers (internal)

**Python-only internal utilities:**
- `cache_test.py` - Tests cache implementation (internal utility)
- `ocr_cache_test.py` - Tests OCR cache (internal utility)
- `device_test.py` - Tests GPU device detection (internal utility)
- `torch_test.py` - Tests PyTorch utilities (internal utility)
- `errors_test.py` - Tests error utilities (internal utility)
- `resource_managers_test.py` - Tests resource managers (internal utility)
- `sync_test.py` - Tests async/sync helpers (internal utility)
- `process_pool_test.py` - Tests process pool (internal utility)
- `pdf_lock_test.py` - Tests PDF locking (internal utility)
- `tmp_test.py` - Tests temp file helpers (internal utility)
- `ref_test.py` - Tests reference counting (internal utility)

**Why DELETE**: These are all internal implementation details. Public API behavior (caching, error handling) IS tested in migrated tests.

---

### 4. FEATURES (2 files) - DELETE ALL

- `gmft_test.py` - Tests `_utils._gmft` internal module
- `token_reduction_test.py` - Tests `_utils._token_reduction` internal module

**Why DELETE**: These test internal implementations. Public API (`TokenReductionConfig`, table extraction) IS tested in migrated tests.

---

### 5. OTHER (6 files) - DELETE ALL

- `conftest.py` - Fixtures for internal tests (not needed)
- `core/registry_test.py` - Tests `ExtractorRegistry` (internal)
- `core/dpi_configuration_test.py` - Tests DPI config helpers (internal)
- `integration/all_extractors_images_test.py` - Tests all extractors with images (internal)
- `multiprocessing/process_manager_test.py` - Tests process manager (internal)
- `benchmarks/extraction_benchmark_test.py` - Benchmarks extractors (internal)

**Why DELETE**: All test internal implementation details.

---

## Recommendation

**DELETE ALL 60 FILES**

None of these files test public API behavior. They all test:
1. Internal classes (extractors, OCR backends)
2. Internal methods (prefixed with `_`)
3. Internal modules (prefixed with `_`)
4. Internal utilities

### What's Already Covered

The 79 migrated tests cover all the PUBLIC API behavior:
- ✅ `extract_file()` / `extract_bytes()` - core/extraction_test.py
- ✅ Batch extraction - core/extraction_batch_test.py
- ✅ Config validation - core/config_test.py
- ✅ Error handling - core/error_handling_test.py
- ✅ MIME type detection - core/mime_types_test.py
- ✅ Chunking - features/chunker_test.py
- ✅ Language detection - features/language_detection_test.py
- ✅ Table extraction - features/table_extraction_test.py
- ✅ API endpoints - api/main_test.py
- ✅ CLI interface - interfaces/cli_test.py
- ✅ Integration tests - integration/*

### Rust Tests Cover Implementation

Internal behavior (PDF parsing, OCR, quality scoring, etc.) should be tested in:
- `crates/kreuzberg/src/*/tests.rs` (Rust unit tests)
- `crates/kreuzberg/tests/` (Rust integration tests)

### Action Plan

1. ✅ Review this document
2. Delete `./tests` directory entirely: `rm -rf ./tests`
3. All future tests go in `packages/python/tests/` and test PUBLIC API only
