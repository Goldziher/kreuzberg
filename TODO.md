# Kreuzberg V4 - Integration Testing TODO

**Status**: Non-OCR Integration Testing Phase
**Last Updated**: 2025-10-18
**Test Status**: 1063+ tests passing ✅ (866 lib + 24 core + 182 integration)
**Coverage**: ~94% (target: 95%)

______________________________________________________________________

## ✅ Completed

### OCR & Format Integration (106 tests)

- ✅ PDF integration tests (20 tests) - `pdf_integration.rs`
- ✅ Office document tests (17 tests) - `office_integration.rs`
- ✅ Image/OCR tests (15 tests) - `image_integration.rs`
- ✅ OCR configuration tests (18 tests) - `ocr_configuration.rs`
- ✅ OCR quality tests (10 tests) - `ocr_quality.rs`
- ✅ Format integration tests (26 tests) - `format_integration.rs`
    - HTML/Web (3 tests)
    - Text/Markdown (2 tests)
    - Data formats (3 tests)
    - Email (1 test)
    - Mixed formats (17 tests)

### Batch Processing (9 tests)

- ✅ `test_batch_extract_file_multiple_formats()` - PDF, DOCX, TXT in one batch
- ✅ `test_batch_extract_file_sync_variant()` - Sync version
- ✅ `test_batch_extract_bytes_multiple()` - Batch from bytes
- ✅ `test_batch_extract_bytes_sync_variant()` - Sync bytes variant
- ✅ `test_batch_extract_empty_list()` - Empty file list
- ✅ `test_batch_extract_one_file_fails()` - Error handling (one fails, others succeed)
- ✅ `test_batch_extract_all_fail()` - All files fail
- ✅ `test_batch_extract_concurrent()` - Parallel processing verification
- ✅ `test_batch_extract_large_batch()` - 50+ files

### Archive Extraction (14 tests)

- ✅ `test_zip_basic_extraction()` - Simple ZIP file
- ✅ `test_zip_multiple_files()` - ZIP with multiple documents
- ✅ `test_zip_nested_directories()` - Directory structure
- ✅ `test_tar_extraction()` - TAR archive
- ✅ `test_tar_gz_extraction()` - TAR.GZ archive (verifies TAR handling)
- ✅ `test_7z_extraction()` - 7Z archive support check
- ✅ `test_nested_archive()` - ZIP inside ZIP
- ✅ `test_archive_mixed_formats()` - PDF + DOCX + images in archive
- ✅ `test_password_protected_archive()` - Encrypted archive (fails gracefully)
- ✅ `test_corrupted_archive()` - Malformed archive handling
- ✅ `test_large_archive()` - 100+ files
- ✅ `test_archive_with_special_characters()` - Unicode filenames
- ✅ `test_empty_archive()` - Zero files
- ✅ `test_archive_extraction_sync()` - Sync variant

### Configuration Features (18 tests)

- ✅ `test_chunking_enabled()` - Text split into chunks
- ✅ `test_chunking_with_overlap()` - Overlap preserved
- ✅ `test_chunking_custom_sizes()` - Custom chunk size/overlap
- ✅ `test_chunking_disabled()` - No chunking when disabled
- ✅ `test_language_detection_single()` - Detect single language
- ✅ `test_language_detection_multiple()` - Multi-language document
- ✅ `test_language_detection_confidence()` - Confidence thresholds
- ✅ `test_language_detection_disabled()` - Feature disabled
- ✅ `test_cache_hit_behavior()` - Second extraction from cache
- ✅ `test_cache_miss_invalidation()` - Cache invalidation
- ✅ `test_custom_cache_directory()` - Non-default cache location
- ✅ `test_cache_disabled()` - Bypass cache
- ✅ `test_token_reduction_aggressive()` - Aggressive mode
- ✅ `test_token_reduction_conservative()` - Conservative mode
- ✅ `test_token_reduction_disabled()` - Feature off
- ✅ `test_quality_processing_enabled()` - Quality scoring
- ✅ `test_quality_threshold_filtering()` - Quality score calculation
- ✅ `test_quality_processing_disabled()` - Feature off

### Email Extraction (10 tests)

- ✅ `test_eml_basic_extraction()` - Subject, from, to, body
- ✅ `test_eml_with_attachments()` - Attachment metadata
- ✅ `test_eml_html_body()` - HTML email
- ✅ `test_eml_plain_text_body()` - Plain text email
- ✅ `test_eml_multipart()` - HTML + plain text parts
- ✅ `test_msg_file_extraction()` - Outlook .msg error handling
- ✅ `test_email_thread()` - Email with quoted replies
- ✅ `test_email_encodings()` - UTF-8 and special characters
- ✅ `test_email_large_attachments()` - Multiple recipients (To, CC, BCC)
- ✅ `test_malformed_email()` - Invalid email structure handling

### Error Handling & Edge Cases (12 tests)

- ✅ `test_truncated_pdf()` - Incomplete PDF
- ✅ `test_corrupted_zip()` - Malformed archive
- ✅ `test_invalid_xml()` - Bad XML syntax
- ✅ `test_corrupted_image()` - Invalid image data
- ✅ `test_empty_file()` - 0 bytes
- ✅ `test_very_large_file()` - Large content (10MB)
- ✅ `test_unicode_filenames()` - Non-ASCII paths
- ✅ `test_special_characters_content()` - Emojis, RTL text, CJK
- ✅ `test_nonexistent_file()` - File not found
- ✅ `test_unsupported_format()` - Unknown file type
- ✅ `test_permission_denied()` - No read access (Unix)
- ✅ `test_file_extension_mismatch()` - MIME type mismatch

### CSV & Spreadsheet Tests (13 tests)

- ✅ `test_csv_basic_extraction()` - Simple CSV
- ✅ `test_csv_with_headers()` - First row as headers
- ✅ `test_csv_custom_delimiter()` - Semicolon delimiters
- ✅ `test_csv_quoted_fields()` - Fields with commas
- ✅ `test_csv_special_characters()` - Unicode characters
- ✅ `test_csv_large_file()` - 10,000 rows (streaming)
- ✅ `test_csv_malformed()` - Inconsistent columns
- ✅ `test_tsv_file()` - Tab-separated values
- ✅ `test_csv_empty()` - Empty CSV file
- ✅ `test_csv_headers_only()` - Only headers
- ✅ `test_csv_blank_lines()` - Blank lines between data
- ✅ `test_csv_numeric_data()` - Numeric formats

### Infrastructure

- ✅ Test helpers module - `tests/helpers/mod.rs`
- ✅ force_ocr implementation for PDF extractor
- ✅ TesseractBackend plugin
- ✅ Comprehensive test documentation
- ✅ Archive MIME types added to core/mime.rs

______________________________________________________________________

## 🎯 HIGH PRIORITY: Non-OCR Integration Testing (17 tests remaining)

### 1. ~~Batch Processing Tests~~ ✅ COMPLETED

### 2. ~~Archive Extraction Tests~~ ✅ COMPLETED

### 3. ~~Configuration Features Tests~~ ✅ COMPLETED

### 4. ~~Email Extraction Tests~~ ✅ COMPLETED

### 5. ~~Error Handling & Edge Cases~~ ✅ COMPLETED

### 6. ~~CSV & Spreadsheet Tests~~ ✅ COMPLETED

______________________________________________________________________

### 7. Pandoc Integration Tests (6 tests) - **OPTIONAL DEPENDENCY**

**Priority**: P2 - Tests optional fallback
**File**: `tests/pandoc_integration.rs` (NEW)
**Time**: 1 hour

- [ ] `test_rst_extraction()` - reStructuredText files
- [ ] `test_latex_extraction()` - .tex files
- [ ] `test_odt_extraction()` - OpenDocument text
- [ ] `test_rtf_extraction()` - Rich Text Format
- [ ] `test_pandoc_not_installed()` - Graceful degradation
- [ ] `test_pandoc_conversion_error()` - Pandoc fails

**Success Criteria**: Pandoc formats work when available, graceful when missing

______________________________________________________________________

### 8. MIME Type Detection Tests (4 tests) - **CORE FEATURE**

**Priority**: P3 - Nice to have
**File**: `tests/mime_detection.rs` (NEW)
**Time**: 30 min - 1 hour

- [ ] `test_mime_detection_by_content()` - Content-based detection
- [ ] `test_mime_detection_by_extension()` - Extension-based
- [ ] `test_mime_mismatch_warning()` - .pdf with DOCX content
- [ ] `test_unknown_mime_type()` - Unsupported format

**Success Criteria**: MIME detection accuracy verified

______________________________________________________________________

## 📊 Summary

### Current Status

- ✅ **Completed**: 182 integration tests (106 OCR/formats + 9 batch + 14 archive + 18 config + 10 email + 12 errors + 13 CSV)
- 🎯 **Target**: 192 integration tests (182 + 10 remaining)
- 📈 **Coverage Goal**: 94% → 95%+
- 🎉 **Progress**: 94.8% complete (182/192)

### Implementation Order

1. ~~**Batch Processing** (9 tests)~~ ✅ **COMPLETED**
1. ~~**Archive Extraction** (14 tests)~~ ✅ **COMPLETED**
1. ~~**Config Features** (18 tests)~~ ✅ **COMPLETED**
1. ~~**Email Extraction** (10 tests)~~ ✅ **COMPLETED**
1. ~~**Error Handling** (12 tests)~~ ✅ **COMPLETED**
1. ~~**CSV/Spreadsheet** (13 tests)~~ ✅ **COMPLETED**
1. **Pandoc Integration** (6 tests) - Optional dependency - **NEXT**
1. **MIME Detection** (4 tests) - Nice to have

### Time Estimates

- **Completed**: 76 tests (~9.5 hours)
- **Remaining**: 10 tests (6 Pandoc + 4 MIME)
- **Estimated time remaining**: 1-1.5 hours
- **Per test average**: 7-9 minutes

______________________________________________________________________

## 🎯 Success Criteria

- ⏳ All core features tested end-to-end
- ✅ **Error handling comprehensive** ← DONE (corrupted files, edge cases, missing files, no panics)
- ✅ **No panics on edge cases** ← DONE (empty files, large files, unicode, special chars)
- ✅ **Batch processing validated** ← DONE
- ✅ **All archive formats supported** ← DONE (ZIP, TAR, 7Z)
- ✅ **Configuration features work correctly** ← DONE (chunking, language detection, caching, token reduction, quality)
- ✅ **Email extraction comprehensive** ← DONE (EML, metadata, HTML/plain text, multipart, encodings)
- ✅ **CSV extraction validated** ← DONE (CSV, TSV, delimiters, quoted fields, large files, malformed)
- ⏳ 95%+ test coverage achieved (currently 94%)
- ⏳ All tests pass in CI/CD

______________________________________________________________________

## 📝 Notes

- Test files available in `test_documents/` (178+ real documents)
- Focus on **behavior** not **implementation**
- Use real documents, avoid mocking
- Test error paths as thoroughly as success paths
- Integration tests complement unit tests
