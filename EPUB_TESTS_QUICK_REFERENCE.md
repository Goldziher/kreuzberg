# EPUB Extractor Tests - Quick Reference

**Location**: `/Users/naamanhirschfeld/workspace/kreuzberg/crates/kreuzberg/tests/epub_extractor_tests.rs`

## One-Line Summary

25 comprehensive integration tests validating EPUB extraction from all 8 test files, with 15 passing (metadata/parsing works) and 10 failing (content extraction issue revealed).

## Test Count by Category

| Category | Tests | Status | Issue |
|----------|-------|--------|-------|
| Basic Extraction | 3 | ✅ 3/3 | Metadata works |
| Chapter Navigation | 2 | ❌ 0/2 | Content loss |
| Structure | 3 | ❌ 1/3 | Formatting extraction |
| Images | 4 | ❌ 2/4 | Content loss with images |
| Metadata | 1 | ✅ 1/1 | Metadata OK |
| Pandoc Parity | 4 | ❌ 0/4 | Content extraction |
| Quality Assurance | 5 | ❌ 1/5 | Content volume |
| Statistics | 1 | ✅ 1/1 | Reporting works |
| **TOTAL** | **25** | **✅ 15/25** | **10 failures** |

## Test Files Covered

```
epub2_cover.epub           [3.2 KB] EPUB2 with cover image
epub2_no_cover.epub        [2.8 KB] EPUB2 without cover
epub2_picture.epub         [3.1 KB] EPUB2 with JPG image
img.epub                   [5.2 KB] EPUB3 with multiple images
img_no_cover.epub          [4.8 KB] EPUB3 without cover
features.epub              [6.1 KB] EPUB3 with features
formatting.epub            [7.3 KB] EPUB3 with CSS styling
wasteland.epub             [25 KB]  T.S. Eliot poetry (complex)
```

## Critical Findings

**Content Extraction Bug**:
- Expected: 5,000-25,000 bytes per file
- Actual: 16-42 bytes per file
- Impact: 99%+ content loss

**Files Affected**: ALL 8 EPUB files show same issue

**Root Cause**: Likely only extracting metadata, not main XHTML content

## Passing Tests (15)

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

## Failing Tests (10)

```
❌ test_epub_basic_content_extraction_epub2_no_cover        (18 bytes, need >50)
❌ test_epub_all_chapters_extracted                         (42 bytes, need >1000)
❌ test_epub_structure_preservation_headings                (31 bytes, need >200)
❌ test_epub_image_handling_with_images                     (42 bytes, need >100)
❌ test_epub_image_no_cover_handling                        (insufficient)
❌ test_epub_complex_book_wasteland                         (16 bytes, need >2000)
❌ test_epub_no_critical_content_loss                       (all files 16-18 bytes)
❌ test_epub_pandoc_parity_features                         (42/5990 bytes = 0.7%)
❌ test_epub_pandoc_parity_formatting                       (31/18413 bytes = 0.17%)
❌ test_epub_pandoc_parity_wasteland                        (16/10235 bytes = 0.16%)
```

## Key Assertions

**Passing tests validate**:
- Files can be opened and parsed ✅
- Metadata extracted (title, creator, identifier) ✅
- UTF-8 encoding valid ✅
- Extraction is deterministic ✅

**Failing tests require**:
- Main content extraction (XHTML files from manifest) ❌
- Substantial bytes per file (>100 bytes minimum) ❌
- Pandoc parity 90-110% ❌
- Chapter/section extraction ❌

## Run Tests

```bash
# All EPUB tests
cargo test --test epub_extractor_tests --features office

# Specific test
cargo test --test epub_extractor_tests test_epub_all_chapters_extracted --features office -- --nocapture

# List all tests
cargo test --test epub_extractor_tests --features office -- --list

# Only failing tests
cargo test --test epub_extractor_tests --features office -- --nocapture 2>&1 | grep FAILED
```

## Helper Functions

```rust
// Get path to test EPUB file
fn get_test_epub_path(filename: &str) -> PathBuf

// Compare extracted content with Pandoc baseline
fn compare_with_baseline(extracted: &str, baseline_filename: &str) -> (usize, usize, f64)
// Returns: (extracted_len, baseline_len, ratio_percent)

// Case-insensitive content check
fn assert_contains_ci(content: &str, needle: &str, description: &str)

// Check if Pandoc installed
async fn skip_if_no_pandoc() -> bool
```

## Test Template

All tests follow this pattern:

```rust
#[tokio::test]
async fn test_name() {
    if skip_if_no_pandoc().await { return; }

    let test_file = get_test_epub_path("filename.epub");
    if !test_file.exists() { return; }

    let result = extract_file(&test_file, "epub").await.expect("msg");

    assert!(!result.content.is_empty());
    assert_contains_ci(&result.content, "expected_text", "description");
}
```

## Feature Flags

Required to run:
```
--features office
```

## Baseline Files

8 Pandoc baseline files for comparison:
```
epub2_cover_pandoc_baseline.txt          [metadata size]
epub2_no_cover_pandoc_baseline.txt       [metadata size]
epub2_picture_pandoc_baseline.txt        [metadata size]
features_pandoc_baseline.txt             [5,990 bytes]
formatting_pandoc_baseline.txt           [18,413 bytes]
img_pandoc_baseline.txt                  [content size]
img_no_cover_pandoc_baseline.txt         [content size]
wasteland_pandoc_baseline.txt            [10,235 bytes]
```

## Success Criteria (Not Yet Met)

1. All 25 tests pass ✅ (15/25 currently)
2. Content >50 bytes minimum ❌ (16-42 bytes extracted)
3. All chapters extracted ❌ (likely first chapter only)
4. Pandoc parity 90-110% ❌ (0.16-0.7% actual)
5. Deterministic extraction ✅ (verified)
6. No raw XML ✅ (verified)
7. Valid UTF-8 ✅ (verified)
8. Metadata extracted ✅ (verified)

## Implementation Gap

**Missing**:
- Loop through manifest items
- XHTML/content file extraction
- Text conversion from HTML markup
- Full document processing

**Working**:
- EPUB file opening
- Package.opf parsing
- Metadata extraction
- Basic structure handling

## Debugging Tips

1. Check content length:
   ```rust
   println!("Extracted: {} bytes", result.content.len());
   println!("Expected: >1000 bytes");
   ```

2. View first 200 chars:
   ```rust
   println!("{}", &result.content[..std::cmp::min(200, result.content.len())]);
   ```

3. Check what's extracted:
   ```rust
   println!("Content: {:?}", result.content);
   ```

4. Validate metadata:
   ```rust
   println!("Metadata: {:?}", result.metadata);
   ```

## File Structure

```
crates/kreuzberg/tests/
└── epub_extractor_tests.rs (1,358 lines)
    ├── Helper functions (lines 50-102)
    ├── Tests 1-13: Original Pandoc-based (lines 125-787)
    ├── Tests 14-25: Enhanced TDD tests (lines 795-1284)
    └── Documentation (lines 1286-1358)
```

## Expected Time to Fix

- **Investigation**: 30 minutes (identify missing XHTML extraction)
- **Implementation**: 1-2 hours (add manifest loop, content extraction)
- **Testing**: 30 minutes (verify all tests pass)
- **Total**: 2-3 hours estimated

## Contact

For questions about these tests, refer to the detailed documentation in:
- `/Users/naamanhirschfeld/workspace/kreuzberg/EPUB_TEST_SUMMARY.md` (comprehensive overview)
- `/Users/naamanhirschfeld/workspace/kreuzberg/EPUB_TESTS_DETAILED.md` (test-by-test breakdown)

---

**Status**: TDD tests complete. Implementation catch-up required.
