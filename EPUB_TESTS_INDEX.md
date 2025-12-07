# EPUB Extractor Tests - Complete Index

## Overview

Comprehensive TDD test suite for EPUB extraction from Kreuzberg library. **25 integration tests** covering all EPUB variations and use cases.

**Status**: Tests complete and operational (15 passing, 10 failing - bug identified)

---

## Quick Links

**Main Test File**: `/Users/naamanhirschfeld/workspace/kreuzberg/crates/kreuzberg/tests/epub_extractor_tests.rs`
- 1,358 lines of code
- 25 comprehensive integration tests
- Full documentation and helper functions

---

## Documentation Files

### 1. EPUB_TESTS_QUICK_REFERENCE.md
**Best for**: Quick overview and lookup
- One-line summary
- Test count by category
- Passing/failing test lists
- Critical findings at a glance
- Run commands
- 2-3 minute read

### 2. EPUB_TEST_SUMMARY.md
**Best for**: Understanding the full scope
- Complete test breakdown by category
- All 8 test files described
- Pandoc baselines explained
- Current test results (15 pass, 10 fail)
- Key findings and recommendations
- 5-10 minute read

### 3. EPUB_TESTS_DETAILED.md
**Best for**: Understanding each individual test
- 25 tests described one-by-one
- What each test validates
- Why each passes or fails
- Assertions and expected values
- Root cause analysis
- Next steps for implementation
- 15-20 minute read

### 4. EPUB_TESTS_RESULTS.md
**Best for**: Understanding current failures
- Detailed failure analysis
- Data points and metrics
- What's working vs what's not
- Root cause hypothesis
- TDD assessment
- Success metrics when fixed
- 10-15 minute read

---

## Test File Statistics

| Metric | Value |
|--------|-------|
| Total Tests | 25 |
| Lines of Code | 1,358 |
| Passing Tests | 15 (60%) |
| Failing Tests | 10 (40%) |
| Test Categories | 8 |
| EPUB Files Tested | 8 |
| Pandoc Baselines | 8 |
| Execution Time | 0.39 seconds |
| Feature Flag | office (required) |

---

## Test Categories

1. **Basic Content Extraction** (3 tests)
   - EPUB2 with cover
   - EPUB2 without cover
   - Basic content validation

2. **Chapter Navigation** (2 tests)
   - Cover detection comparison
   - All chapters extraction

3. **Structure Preservation** (3 tests)
   - EPUB3 features
   - Formatting/styling
   - Heading preservation

4. **Image Handling** (4 tests)
   - Multiple image formats
   - Cover with images
   - Without cover handling

5. **Metadata Extraction** (1 test)
   - Dublin Core fields
   - Creator/author
   - Identifier/title

6. **Pandoc Parity** (4 tests)
   - Baseline compliance
   - Features baseline
   - Formatting baseline
   - Wasteland baseline

7. **Quality Assurance** (5 tests)
   - Content quality
   - Deterministic extraction
   - No content loss
   - Unicode validity

8. **Statistics & Reporting** (1 test)
   - Comprehensive statistics
   - Metadata counts

---

## Test Files Covered

```
epub2_cover.epub           [3.2 KB]  EPUB2 with cover image
epub2_no_cover.epub        [2.8 KB]  EPUB2 without cover
epub2_picture.epub         [3.1 KB]  EPUB2 with JPG image
img.epub                   [5.2 KB]  EPUB3 with multiple images
img_no_cover.epub          [4.8 KB]  EPUB3 without cover
features.epub              [6.1 KB]  EPUB3 with features
formatting.epub            [7.3 KB]  EPUB3 with CSS styling
wasteland.epub             [25 KB]   T.S. Eliot poetry (complex)
```

---

## Current Results Summary

### Passing Tests (15)

Essential functionality working:
- File parsing and opening
- Metadata extraction (title, creator, identifier)
- Pandoc baseline reading
- UTF-8 validation
- Deterministic extraction
- Statistics reporting

```
✅ test_epub2_cover_extraction
✅ test_epub2_no_cover_extraction
✅ test_epub2_cover_detection_difference
✅ test_epub2_picture_extraction
✅ test_epub_image_extraction
✅ test_epub_image_no_cover_extraction
✅ test_epub_features_extraction
✅ test_epub_formatting_extraction
✅ test_epub_wasteland_extraction
✅ test_epub_comprehensive_metadata
✅ test_epub_content_quality
✅ test_epub_pandoc_baseline_compliance
✅ test_epub_extraction_deterministic
✅ test_epub_unicode_validity
✅ test_epub_extraction_statistics
```

### Failing Tests (10)

Content extraction not working:
- Only 16-42 bytes extracted per file (should be 2,000-25,000)
- Main XHTML content files not being processed
- Likely only extracting metadata, not content

```
❌ test_epub_basic_content_extraction_epub2_no_cover (18 bytes, need >50)
❌ test_epub_all_chapters_extracted (42 bytes, need >1000)
❌ test_epub_structure_preservation_headings (31 bytes, need >200)
❌ test_epub_image_handling_with_images (42 bytes, need >100)
❌ test_epub_image_no_cover_handling (insufficient)
❌ test_epub_complex_book_wasteland (16 bytes, need >2000)
❌ test_epub_no_critical_content_loss (all files 16-42 bytes)
❌ test_epub_pandoc_parity_features (42/5990 bytes = 0.7%)
❌ test_epub_pandoc_parity_formatting (31/18413 bytes = 0.17%)
❌ test_epub_pandoc_parity_wasteland (16/10235 bytes = 0.16%)
```

---

## Critical Bug Identified

**Issue**: 99%+ content loss in EPUB extraction
- Expected: 5,000-25,000 bytes per file
- Actual: 16-42 bytes per file
- Affected: All 8 EPUB files
- Root cause: Main content files (XHTML) not being extracted

**Primary validation test**: `test_epub_all_chapters_extracted` (Test 15)

---

## How to Use

### Run All Tests
```bash
cd /Users/naamanhirschfeld/workspace/kreuzberg
cargo test --test epub_extractor_tests --features office
```

### Run Specific Test
```bash
cargo test --test epub_extractor_tests test_epub_all_chapters_extracted --features office -- --nocapture
```

### Run Only Failing Tests
```bash
cargo test --test epub_extractor_tests --features office | grep FAILED
```

### List All Tests
```bash
cargo test --test epub_extractor_tests --features office -- --list
```

---

## Test Template

All tests follow consistent pattern:

```rust
#[tokio::test]
async fn test_name() {
    // Skip if Pandoc not available
    if skip_if_no_pandoc().await { return; }

    // Get test file
    let test_file = get_test_epub_path("filename.epub");
    if !test_file.exists() { return; }

    // Extract content
    let result = extract_file(&test_file, "epub")
        .await
        .expect("Should extract");

    // Validate
    assert!(!result.content.is_empty());
    assert_contains_ci(&result.content, "expected", "description");
}
```

---

## Helper Functions

All helpers located in test file (lines 50-102):

```rust
/// Get path to test EPUB file
fn get_test_epub_path(filename: &str) -> PathBuf

/// Get path to Pandoc baseline
fn get_baseline_path(filename: &str) -> PathBuf

/// Compare with baseline - returns (extracted_len, baseline_len, ratio%)
fn compare_with_baseline(extracted: &str, baseline: &str) -> (usize, usize, f64)

/// Case-insensitive content validation
fn assert_contains_ci(content: &str, needle: &str, description: &str)

/// Check Pandoc installation
async fn skip_if_no_pandoc() -> bool
```

---

## Test Organization in File

```
epub_extractor_tests.rs (1,358 lines)
├── Header & feature flags (lines 1-45)
├── Helper functions (lines 47-102)
├── Tests 1-13: Original Pandoc-based (lines 115-787)
│   ├── EPUB2 Cover Tests (1-4)
│   ├── Image Handling (5-6)
│   ├── Features & Formatting (7-8)
│   ├── Wasteland Poetry (9)
│   ├── Metadata Extraction (10)
│   ├── Content Quality (11)
│   ├── Baseline Compliance (12)
│   └── Statistics (13)
├── Tests 14-25: Enhanced TDD (lines 795-1284)
│   ├── Critical Bug Tests (14-15)
│   ├── Structure Tests (16)
│   ├── Image Tests (17-18)
│   ├── Complex Book Test (19)
│   ├── Pandoc Parity Tests (20-22)
│   ├── Deterministic Test (23)
│   ├── No Loss Test (24)
│   └── Unicode Test (25)
└── Documentation (lines 1286-1358)
```

---

## Related Resources

### Test Documents
Location: `/Users/naamanhirschfeld/workspace/kreuzberg/test_documents/epub/`

8 EPUB files + 8 Pandoc baseline files

### Source Code
Likely location for EPUB extractor:
- `crates/kreuzberg/src/extraction/`
- `crates/kreuzberg/src/extractors/`

### Documentation
- LaTeX tests: `crates/kreuzberg/tests/latex_extractor_tests.rs` (similar structure)
- RTF tests: `crates/kreuzberg/tests/rtf_extractor_tests.rs` (reference implementation)

---

## Success Criteria

When EPUB extraction is fixed:

1. **All 25 tests pass** ✅
2. **Content extraction >50 bytes minimum** per file
3. **All chapters extracted** (validate with >1000 bytes for multi-chapter)
4. **Pandoc parity 90-110%** for key files
5. **Deterministic extraction** (same input = same output)
6. **No raw XML/HTML** in output
7. **Valid UTF-8 encoding** throughout
8. **Metadata extracted** (title, creator, identifier, etc.)

---

## Quick Statistics

| Test Name | Type | Status | Issue |
|-----------|------|--------|-------|
| EPUB2 Cover | extraction | ✅ | Metadata only |
| EPUB2 No Cover | extraction | ✅ | Metadata only |
| Cover Difference | comparison | ✅ | Parsing OK |
| Picture | images | ✅ | Metadata OK |
| Images | multimedia | ✅ | Metadata OK |
| Images No Cover | multimedia | ✅ | Metadata OK |
| Features | structure | ✅ | Parsing OK |
| Formatting | styling | ✅ | Parsing OK |
| Wasteland | complex | ✅ | File loads |
| Metadata | extraction | ✅ | Fields OK |
| Content Quality | validation | ✅ | Non-empty |
| Baseline | compliance | ✅ | Format OK |
| Statistics | reporting | ✅ | Counts work |
| **Basic Content** | **content** | ❌ | **18 bytes** |
| **All Chapters** | **content** | ❌ | **42 bytes** |
| **Headings** | **structure** | ❌ | **31 bytes** |
| **Image Content** | **content** | ❌ | **42 bytes** |
| **Image No Cover** | **content** | ❌ | **Low bytes** |
| **Complex Book** | **content** | ❌ | **16 bytes** |
| **No Loss** | **validation** | ❌ | **All low** |
| **Parity Features** | **parity** | ❌ | **0.7%** |
| **Parity Format** | **parity** | ❌ | **0.17%** |
| **Parity Waste** | **parity** | ❌ | **0.16%** |
| Deterministic | validation | ✅ | OK |
| Unicode | validation | ✅ | UTF-8 OK |

---

## Recommended Reading Order

1. **Start here**: `EPUB_TESTS_QUICK_REFERENCE.md` (3 min)
   - Get oriented with quick overview

2. **Then read**: `EPUB_TEST_SUMMARY.md` (8 min)
   - Understand full scope and categorization

3. **If investigating bug**: `EPUB_TESTS_RESULTS.md` (12 min)
   - See detailed failure analysis

4. **For implementation**: `EPUB_TESTS_DETAILED.md` (20 min)
   - Understand each test individually

5. **Reference**: This file and main test file
   - Navigate and understand structure

---

## Quick Facts

- **Created**: 2025-12-07
- **Test Count**: 25 comprehensive integration tests
- **Test File**: 1,358 lines of code
- **Pass Rate**: 60% (15 passing, 10 failing)
- **Execution Time**: 0.39 seconds
- **Files Tested**: 8 EPUB variants
- **Bug Identified**: 99%+ content extraction loss
- **Status**: Tests complete, implementation needs work
- **Effort to Fix**: Estimated 2-3 hours

---

## Contact & Support

For questions about the test suite, refer to:
1. This index for navigation
2. Quick reference for overview
3. Detailed docs for specific tests
4. Main test file for code

For implementation help, look at:
1. Similar extractor tests (LaTeX, RTF)
2. Pandoc integration code
3. EPUB specification details

---

End of index. Start with `EPUB_TESTS_QUICK_REFERENCE.md` for 3-minute overview.
