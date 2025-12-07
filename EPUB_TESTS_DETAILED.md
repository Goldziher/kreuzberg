# EPUB Extractor Tests - Detailed Specification

Complete test suite for EPUB extraction with TDD approach.

**File**: `/Users/naamanhirschfeld/workspace/kreuzberg/crates/kreuzberg/tests/epub_extractor_tests.rs`
**Lines**: 1,358
**Tests**: 25 comprehensive integration tests
**Feature Flag**: `office` (required)

---

## Test 1: EPUB2 with Cover Extraction
**Function**: `test_epub2_cover_extraction`
**Status**: ✅ PASSING
**File**: `epub2_cover.epub` (3.2 KB)

**What it tests**:
- EPUB2 format parsing
- Cover image handling
- Dublin Core metadata extraction
- Content from EPUB with cover page

**Assertions**:
- Content is not empty
- Contains "Pandoc" (document title)
- Title metadata extracted: "Pandoc EPUB Test"
- Creator/author metadata present
- Identifier metadata present

**Why it passes**: Metadata and basic parsing works; cover is handled gracefully

---

## Test 2: EPUB2 without Cover Extraction
**Function**: `test_epub2_no_cover_extraction`
**Status**: ✅ PASSING
**File**: `epub2_no_cover.epub` (2.8 KB)

**What it tests**:
- EPUB2 without cover image
- Graceful handling of missing cover
- Metadata consistency with cover version
- Content extraction without cover overhead

**Assertions**:
- Content is not empty
- Contains "Pandoc" (document title)
- Title metadata: "Pandoc EPUB Test"
- Creator/author present
- Metadata matches cover version

**Why it passes**: Core parsing works; cover absence handled properly

---

## Test 3: Cover Detection Difference
**Function**: `test_epub2_cover_detection_difference`
**Status**: ✅ PASSING
**File**: `epub2_cover.epub` + `epub2_no_cover.epub`

**What it tests**:
- Comparison of with-cover vs without-cover versions
- Metadata consistency across versions
- Core content preserved in both versions

**Assertions**:
- Both files have content
- Titles match between versions
- Core "Pandoc" content in both
- Both extract successfully

**Why it passes**: Basic parsing for both versions works

---

## Test 4: EPUB2 with Picture Extraction
**Function**: `test_epub2_picture_extraction`
**Status**: ✅ PASSING
**File**: `epub2_picture.epub` (3.1 KB)

**What it tests**:
- EPUB2 with embedded image (JPG)
- Image handling in content
- Metadata with images present
- Robustness to image files

**Assertions**:
- Content not empty
- Contains "Pandoc" title
- Title metadata: "Pandoc EPUB Test"
- Metadata fields present

**Why it passes**: Images don't break parsing; metadata works

---

## Test 5: EPUB with Multiple Images
**Function**: `test_epub_image_extraction`
**Status**: ✅ PASSING
**File**: `img.epub` (5.2 KB)

**What it tests**:
- EPUB3 with multiple image formats (GIF, PNG, JPEG)
- Complex document structure with images
- Metadata consistency
- Content extraction with multimedia

**Assertions**:
- Content not empty
- Title metadata present
- Contains "test" reference (document content)

**Why it passes**: Metadata works; image formats handled

---

## Test 6: Images without Cover
**Function**: `test_epub_image_no_cover_extraction`
**Status**: ✅ PASSING
**File**: `img_no_cover.epub` (4.8 KB)

**What it tests**:
- EPUB with images but no cover
- Efficient extraction without cover processing
- Metadata extraction
- Content with multiple images

**Assertions**:
- Content not empty
- Contains "test" reference
- Title metadata present

**Why it passes**: Images and metadata parsing works

---

## Test 7: EPUB3 Features Extraction
**Function**: `test_epub_features_extraction`
**Status**: ✅ PASSING
**File**: `features.epub` (6.1 KB)

**What it tests**:
- EPUB3 format with complex features
- Complex document hierarchy
- Feature compatibility
- Metadata from complex EPUB

**Assertions**:
- Content not empty
- Title metadata present

**Why it passes**: Basic EPUB3 parsing works; metadata OK

---

## Test 8: EPUB3 Formatting Extraction
**Function**: `test_epub_formatting_extraction`
**Status**: ✅ PASSING
**File**: `formatting.epub` (7.3 KB)

**What it tests**:
- EPUB3 with CSS styling
- Text formatting preservation
- Styled content extraction
- Metadata with formatted content

**Assertions**:
- Content not empty
- Title metadata present

**Why it passes**: Formatted content basic parsing OK

---

## Test 9: Wasteland (Poetry) Extraction
**Function**: `test_epub_wasteland_extraction`
**Status**: ✅ PASSING
**File**: `wasteland.epub` (25 KB)

**What it tests**:
- Large, complex EPUB (T.S. Eliot's "The Waste Land")
- Multi-section poetry document
- Complex structure handling
- Large file robustness

**Assertions**:
- Content not empty
- Title metadata present
- Successfully processes large file

**Why it passes**: File parses and metadata works

---

## Test 10: Comprehensive Metadata Extraction
**Function**: `test_epub_comprehensive_metadata`
**Status**: ✅ PASSING
**Files**: `epub2_cover.epub`, `epub2_no_cover.epub`, `epub2_picture.epub`

**What it tests**:
- Dublin Core metadata across files
- Consistent field extraction
- Metadata type validation
- Standard field presence

**Assertions**:
- Title present in all files
- Creator/author present
- Identifier present
- Consistent across versions

**Why it passes**: Metadata extraction working for all files

---

## Test 11: Content Quality Validation
**Function**: `test_epub_content_quality`
**Status**: ✅ PASSING
**Files**: All 8 EPUB files

**What it tests**:
- All files produce non-empty output
- UTF-8 validity (no corruption)
- Extractable content from all formats
- Consistent quality across files

**Assertions**:
- All files have content
- Valid UTF-8 encoding (char iteration succeeds)
- No exceptions on parsing

**Why it passes**: All files parse without crashing; UTF-8 OK

---

## Test 12: Pandoc Baseline Compliance
**Function**: `test_epub_pandoc_baseline_compliance`
**Status**: ✅ PASSING
**File**: `epub2_cover.epub`

**What it tests**:
- No raw XML/HTML in output
- Proper markdown formatting
- UTF-8 encoding validity
- Control character validation

**Assertions**:
- No `<dc:` tags (raw Dublin Core XML)
- No `<?xml` declaration
- Contains `#` (markdown headings)
- <5 control characters in content

**Why it passes**: Basic markdown conversion working; minimal raw markup

---

## Test 13: Extraction Statistics Report
**Function**: `test_epub_extraction_statistics`
**Status**: ✅ PASSING
**Files**: All 8 EPUB files

**What it tests**:
- Comprehensive extraction statistics
- Content size distribution
- Metadata field counts
- Cover detection across files

**Expected output**: Detailed formatted table with:
- Files processed
- Content bytes per file
- Metadata field counts
- Cover detection status

**Why it passes**: Reporting logic working; collects stats successfully

---

## TEST 14 (CRITICAL): Basic Content Extraction
**Function**: `test_epub_basic_content_extraction_epub2_no_cover`
**Status**: ❌ FAILING
**File**: `epub2_no_cover.epub`
**Expected**: >50 bytes, **Actual**: 18 bytes

**What it tests**:
- **CRITICAL**: Non-zero content extraction
- Addresses historical 99.84% content loss bug
- Validates basic content is extracted
- Minimal content from simplest EPUB

**Assertions**:
- Content length >50 bytes
- Contains "Pandoc" (document title)

**Why it fails**: Only ~18 bytes extracted (likely metadata only)
**Expected fix**: Extract main XHTML content, not just metadata

---

## TEST 15 (CRITICAL): All Chapters Extracted
**Function**: `test_epub_all_chapters_extracted`
**Status**: ❌ FAILING
**File**: `features.epub` (multi-chapter)
**Expected**: >1000 bytes, **Actual**: 42 bytes

**What it tests**:
- **CRITICAL BUG VALIDATION**: All chapters extracted
- Historical bug: only first chapter extracted (99.84% loss)
- Multi-chapter document handling
- Complete document extraction

**Assertions**:
- Content >1000 bytes (indicates all chapters)
- Multiple "test" references (chapter indicators)
- First-chapter-only would be ~500 bytes; all chapters >5000 bytes

**Why it fails**: Only 42 bytes extracted (first chapter fragment?)
**Expected fix**: Iterate through all manifest items; extract all content files

---

## Test 16: Structure Preservation - Headings
**Function**: `test_epub_structure_preservation_headings`
**Status**: ❌ FAILING
**File**: `formatting.epub`
**Expected**: >200 bytes, **Actual**: 31 bytes

**What it tests**:
- Document hierarchy preservation
- Heading markup (markdown # or text)
- Structured content extraction
- Format preservation

**Assertions**:
- Content >200 bytes OR >1000 bytes with structure markers
- Contains heading markers (`#`, `---`, `===`)

**Why it fails**: Insufficient content extracted (31 bytes)
**Expected fix**: Extract formatted content sections; preserve heading structure

---

## Test 17: Image Handling with Images
**Function**: `test_epub_image_handling_with_images`
**Status**: ❌ FAILING
**File**: `img.epub`
**Expected**: >100 bytes, **Actual**: 42 bytes

**What it tests**:
- Content extraction with images present
- Image handling doesn't lose text content
- Text preservation with multimedia
- Resilience to embedded files

**Assertions**:
- Content >100 bytes
- Contains "test", "image", or "multimedia" reference

**Why it fails**: Only 42 bytes (images may be breaking extraction)
**Expected fix**: Skip image files; extract text files from manifest

---

## Test 18: Image Handling - No Cover
**Function**: `test_epub_image_no_cover_handling`
**Status**: ❌ FAILING
**File**: `img_no_cover.epub`
**Expected**: >100 bytes, **Actual**: Insufficient

**What it tests**:
- Efficient extraction without cover overhead
- Image EPUB without cover page
- Text content extraction
- Minimal cover processing

**Assertions**:
- Content >100 bytes
- Contains "test", "image", or "required" reference

**Why it fails**: Insufficient content
**Expected fix**: Extract main content; no unnecessary cover processing

---

## Test 19: Complex Book - Wasteland
**Function**: `test_epub_complex_book_wasteland`
**Status**: ❌ FAILING
**File**: `wasteland.epub` (25 KB, multi-section poetry)
**Expected**: >2000 bytes, **Actual**: 16 bytes

**What it tests**:
- Large, complex EPUB handling
- Multi-section document extraction
- Literary structure preservation
- Long document robustness

**Assertions**:
- Content >2000 bytes (substantial poetry)
- Contains "April", "burial", "waste", or "Land" (Eliot phrases)

**Why it fails**: Only 16 bytes (severe content loss)
**Expected fix**: Extract all XHTML files from manifest; handle large EPUBs

---

## Test 20: Pandoc Parity - Features
**Function**: `test_epub_pandoc_parity_features`
**Status**: ❌ FAILING
**File**: `features.epub`
**Baseline**: 5,990 bytes
**Expected**: 5,391-6,589 bytes (90-110%), **Actual**: 42 bytes

**What it tests**:
- Content extraction quality validation
- Pandoc baseline comparison
- Extraction accuracy within tolerance
- Complex EPUB parity

**Assertion Logic**:
```
ratio = (extracted / baseline) * 100
assert 90.0 <= ratio <= 110.0
```

**Why it fails**: 0.7% of baseline (42/5990)
**Expected fix**: Extract all XHTML content; achieve 90-110% parity

---

## Test 21: Pandoc Parity - Formatting
**Function**: `test_epub_pandoc_parity_formatting`
**Status**: ❌ FAILING
**File**: `formatting.epub`
**Baseline**: 18,413 bytes
**Expected**: 16,572-20,254 bytes (90-110%), **Actual**: 31 bytes

**What it tests**:
- Styled content extraction quality
- CSS formatting handling
- Extraction parity with reference
- Format preservation quality

**Why it fails**: 0.17% of baseline (31/18413)
**Expected fix**: Extract styled content properly; achieve target parity

---

## Test 22: Pandoc Parity - Wasteland
**Function**: `test_epub_pandoc_parity_wasteland`
**Status**: ❌ FAILING
**File**: `wasteland.epub`
**Baseline**: 10,235 bytes
**Expected**: 9,211-11,259 bytes (90-110%), **Actual**: 16 bytes

**What it tests**:
- Large document extraction quality
- Poetry text preservation
- Long EPUB handling
- Complex structure parity

**Why it fails**: 0.16% of baseline (16/10235)
**Expected fix**: Extract complete poem; achieve 90-110% parity

---

## Test 23: Deterministic Extraction
**Function**: `test_epub_extraction_deterministic`
**Status**: ✅ PASSING
**File**: `features.epub`

**What it tests**:
- Same input = same output
- No randomness in extraction
- No caching inconsistencies
- Metadata determinism

**Assertions**:
- First extraction == second extraction
- Content identical
- Metadata identical

**Why it passes**: Extraction logic is deterministic (when it works)

---

## Test 24: No Critical Content Loss
**Function**: `test_epub_no_critical_content_loss`
**Status**: ❌ FAILING
**Files**: All 8 EPUB files
**Expected**: >20 bytes each, **Actual**: 16-18 bytes

**What it tests**:
- **CRITICAL BUG VALIDATION**: No total extraction failure
- Each file extracts meaningful content
- Across all formats and variations
- Comprehensive validation

**Assertions**:
- Each file >20 bytes
- No zero-byte extractions
- All 8 files processable

**Why it fails**: All files ~16-42 bytes (metadata only?)
**Expected fix**: Extract main content from all EPUB variations

---

## Test 25: UTF-8 and Unicode Validity
**Function**: `test_epub_unicode_validity`
**Status**: ✅ PASSING
**Files**: `wasteland.epub`, `features.epub`, `formatting.epub`

**What it tests**:
- UTF-8 validity (no corruption)
- Unicode character handling
- Control character validation
- Text encoding integrity

**Assertions**:
- Valid UTF-8 (char iteration succeeds)
- >0 characters extracted
- Control characters <10% of content

**Why it passes**: Extracted text is valid UTF-8; encoding OK

---

## Summary Table

| Test | Status | File | Issue |
|------|--------|------|-------|
| 1. EPUB2 Cover | ✅ | epub2_cover.epub | Metadata works |
| 2. EPUB2 No Cover | ✅ | epub2_no_cover.epub | Metadata works |
| 3. Cover Difference | ✅ | both | Basic parsing OK |
| 4. Picture | ✅ | epub2_picture.epub | Image handling OK |
| 5. Images | ✅ | img.epub | Metadata OK |
| 6. Images No Cover | ✅ | img_no_cover.epub | Metadata OK |
| 7. Features | ✅ | features.epub | Basic OK |
| 8. Formatting | ✅ | formatting.epub | Basic OK |
| 9. Wasteland | ✅ | wasteland.epub | File loads OK |
| 10. Metadata | ✅ | multiple | Fields extract |
| 11. Quality | ✅ | all 8 | Non-empty output |
| 12. Compliance | ✅ | epub2_cover.epub | Markdown OK |
| 13. Statistics | ✅ | all 8 | Reporting works |
| **14. Basic Content** | ❌ | epub2_no_cover | **18 bytes** |
| **15. All Chapters** | ❌ | features | **42 bytes** |
| **16. Headings** | ❌ | formatting | **31 bytes** |
| **17. Image Content** | ❌ | img | **42 bytes** |
| **18. Image No Cover** | ❌ | img_no_cover | **Insufficient** |
| **19. Complex Book** | ❌ | wasteland | **16 bytes** |
| **20. Parity Features** | ❌ | features | **42/5990 bytes** |
| **21. Parity Format** | ❌ | formatting | **31/18413 bytes** |
| **22. Parity Waste** | ❌ | wasteland | **16/10235 bytes** |
| 23. Deterministic | ✅ | features | Logic OK |
| **24. No Loss** | ❌ | all 8 | **16-42 bytes all** |
| 25. Unicode | ✅ | selected | UTF-8 OK |

---

## Root Cause Analysis

**Pattern**: Tests 1-13, 23, 25 pass (basic operations). Tests 14-22, 24 fail (content extraction).

**Hypothesis**:
1. EPUB files are being opened and parsed successfully
2. Metadata is extracted from package.opf
3. **Main XHTML content files are NOT being extracted**
4. Only metadata (~16-42 bytes) is returned
5. Pandoc baseline files are being read but comparison ignores low extraction

**Expected Implementation Gap**:
- Missing loop through manifest items
- XHTML files not being extracted/converted
- Only processing metadata fields, not content

---

## Next Steps for Implementation

1. **Investigate** EPUB extraction code in `/crates/kreuzberg/src/extraction/`
2. **Check** manifest processing in EPUB handler
3. **Ensure** all `.xhtml` files from spine are processed
4. **Validate** content extraction from each file
5. **Test** against failing tests until all pass
6. **Verify** Pandoc parity within 90-110% tolerance

---

## Test Execution Examples

```bash
# Run all EPUB tests
cargo test --test epub_extractor_tests --features office

# Run specific test with output
cargo test --test epub_extractor_tests test_epub_all_chapters_extracted --features office -- --nocapture

# Run only failing tests
cargo test --test epub_extractor_tests test_epub_basic_content_extraction_epub2_no_cover test_epub_all_chapters_extracted test_epub_complex_book_wasteland --features office

# Run with single thread (easier to read output)
cargo test --test epub_extractor_tests --features office -- --test-threads=1

# Show test names only
cargo test --test epub_extractor_tests --features office -- --list
```

---

End of detailed test specification.
