# Test Migration Status for V4

## Summary

- **Original tests**: 79 files migrated from root
- **Deleted (private modules)**: 16 files ✅
- **Remaining**: 63 files
  - **Working**: 22 files (collected successfully, may pass/fail)
  - **Need fixes**: 13 files (collection errors)
  - **Unknown status**: 28 files (need manual review - no kreuzberg imports)

---

## Files Needing Fixes (13 collection errors)

### Category 1: Missing Python Modules (5 files)

These test Python-specific features that haven't been implemented yet in the v4 Python package:

1. **tests/api/** - `ModuleNotFoundError: No module named 'kreuzberg._api'`
   - Status: API server not implemented in v4 Python package yet
   - Action: Implement API server or skip these tests

2. **tests/interfaces/cli_test.py** - Missing `kreuzberg.cli`
   - Status: CLI not implemented in v4 Python package yet
   - Action: Implement CLI proxy or skip

3. **tests/interfaces/mcp_server_test.py** - Missing `kreuzberg._mcp`
   - Status: MCP server not implemented in v4 Python package yet
   - Action: Implement MCP server or skip

### Category 2: Missing V4 APIs - Need Rewrites (10 files)

These test public behavior but use v3 APIs that changed or were removed:

4. **tests/core/config_test.py**
   - Issue: Imports `build_extraction_config_from_dict` (doesn't exist in v4)
   - Action: Remove or rewrite - configs are now simple dataclasses

5. **tests/core/exceptions_test.py**
   - Issue: Imports `kreuzberg.exceptions` module (moved to Rust)
   - Action: Check if exceptions are exposed, otherwise delete

6. **tests/core/extraction_batch_test.py**
   - Issue: Imports `batch_extract_file` (renamed to `batch_extract_files`)
   - Action: Simple rename fix

7. **tests/core/extraction_test.py**
   - Issue: Imports `EntityExtractionConfig`, `KeywordExtractionConfig`
   - Action: These are Python-only features, need to check if they exist

8. **tests/core/image_ocr_result_test.py**
   - Issue: Imports `ExtractedImage`, `ImageOCRResult`
   - Action: Check if these types exist in v4, otherwise delete

9. **tests/core/types_test.py**
   - Issue: Imports `ExtractedImage`, `EasyOCRConfig`
   - Action: Check if these types exist in v4

10. **tests/gmft/config_validation_test.py**
    - Issue: Imports `TableExtractionConfig`
    - Action: GMFT/vision tables is Python-only, check if implemented

11. **tests/integration/dpi_integration_test.py**
    - Issue: Imports `TesseractConfig`
    - Action: Rename to `OcrConfig`

12. **tests/integration/regression_test.py**
    - Issue: Imports `batch_extract_file`, `PSMMode`, `TesseractConfig`
    - Action: Multiple renames needed

13. **tests/integration/token_reduction_integration_test.py**
    - Issue: Imports `EntityExtractionConfig` (line 118 usage)
    - Action: Remove entity extraction test or make it conditional

---

## Quick Fixes

### Simple Renames (3 files)

```python
# OLD v3 API → NEW v4 API
batch_extract_file → batch_extract_files
TesseractConfig → OcrConfig
PSMMode → ? (check if exists)
```

**Files:**
- tests/core/extraction_batch_test.py
- tests/integration/dpi_integration_test.py
- tests/integration/regression_test.py

### Remove Missing Configs (2 files)

These configs don't exist in v4's public API (Python-only features):

```python
# Remove these imports or make conditional:
EntityExtractionConfig  # Spacy-based entity extraction
KeywordExtractionConfig # KeyBERT-based keyword extraction
```

**Files:**
- tests/core/extraction_test.py
- tests/integration/token_reduction_integration_test.py

---

## Recommended Next Steps

1. **Quick wins**: Fix the 3 simple rename issues first
2. **Check v4 API**: Verify what types/configs actually exist in v4
3. **Python features**: Decide which Python-only features to implement (API, CLI, MCP, entities, keywords)
4. **Delete or skip**: Remove tests for features we're not implementing in v4

---

## V4 Public API Reference

Current exports from `kreuzberg/__init__.py`:

**Config Types:**
- ChunkingConfig
- ExtractionConfig
- ImageExtractionConfig
- LanguageDetectionConfig
- OcrConfig
- PdfConfig
- TokenReductionConfig

**Result Types:**
- ExtractionResult
- ExtractedTable

**Functions:**
- extract_file_sync, extract_bytes_sync
- batch_extract_files_sync, batch_extract_bytes_sync
- extract_file, extract_bytes
- batch_extract_files, batch_extract_bytes
- detect_mime_type, validate_mime_type
- register_ocr_backend, list_ocr_backends, unregister_ocr_backend

**Missing from v3:**
- EntityExtractionConfig, KeywordExtractionConfig (Python features not exposed yet)
- ExtractedImage, ImageOCRResult (types - need to check)
- TableExtractionConfig (GMFT/vision tables - need to check)
- TesseractConfig (renamed to OcrConfig)
- PSMMode (OCR page segmentation mode - need to check)
- build_extraction_config_from_dict (helper function - not needed with dataclasses)
- exceptions module (errors now in Rust)
