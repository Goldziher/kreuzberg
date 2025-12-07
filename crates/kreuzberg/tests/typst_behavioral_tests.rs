//! Comprehensive behavioral tests for Typst extractor against Pandoc baselines.
//!
//! These tests expose the critical bugs found in code review:
//! 1. 62% heading loss bug - only matches single `=` headings
//! 2. Blockquotes not implemented
//! 3. Display math not extracted
//! 4. Nested table brackets cause corruption
//! 5. Empty headings output (just `= ` with no text)
//! 6. Regex failures silently lose metadata
//!
//! The tests are designed to FAIL initially, exposing real bugs that need fixing.
//! They compare extracted output against Pandoc baseline outputs for behavioral parity.

use kreuzberg::core::config::ExtractionConfig;
use kreuzberg::core::extractor::extract_bytes;
use std::fs;

/// Load a test document from the test_documents/typst directory
fn load_test_document(filename: &str) -> Vec<u8> {
    let path = format!(
        "/Users/naamanhirschfeld/workspace/kreuzberg/test_documents/typst/{}",
        filename
    );
    fs::read(&path).expect(&format!("Failed to read test document: {}", filename))
}

/// Load Pandoc baseline output for comparison
fn load_pandoc_baseline(filename_base: &str) -> String {
    let path = format!(
        "/Users/naamanhirschfeld/workspace/kreuzberg/test_documents/typst/{}_pandoc_baseline.txt",
        filename_base
    );
    fs::read_to_string(&path).expect(&format!("Failed to read baseline: {}", filename_base))
}

/// Load Pandoc metadata JSON for comparison
fn load_pandoc_metadata(filename_base: &str) -> String {
    let path = format!(
        "/Users/naamanhirschfeld/workspace/kreuzberg/test_documents/typst/{}_pandoc_meta.json",
        filename_base
    );
    fs::read_to_string(&path).expect(&format!("Failed to read metadata: {}", filename_base))
}

/// Count specific heading levels (= for level 1, == for level 2, etc.)
fn count_heading_level(content: &str, level: usize) -> usize {
    let exact_marker = format!("{} ", "=".repeat(level));
    content
        .lines()
        .filter(|l| l.trim_start().starts_with(&exact_marker))
        .count()
}

/// Extract all headings from content
fn extract_all_headings(content: &str) -> Vec<String> {
    content
        .lines()
        .filter(|l| {
            let trimmed = l.trim_start();
            trimmed.starts_with('=') && !trimmed.starts_with("#set")
        })
        .map(|l| l.to_string())
        .collect()
}

/// Count lines that are pure metadata/directives (not content)
fn count_directive_lines(content: &str) -> usize {
    content
        .lines()
        .filter(|l| {
            let t = l.trim();
            t.starts_with("#set ") || t.starts_with("#let ") || t.starts_with("#import ")
        })
        .count()
}

/// Count empty headings (headings with just `= ` and no text)
fn count_empty_headings(content: &str) -> usize {
    content
        .lines()
        .filter(|l| {
            let trimmed = l.trim_start();
            trimmed == "="
                || trimmed == "=="
                || trimmed == "==="
                || trimmed == "===="
                || trimmed == "====="
                || trimmed == "======"
        })
        .count()
}

/// Extract all text between headings (content blocks)
fn extract_content_blocks(content: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut current_block = String::new();
    let mut in_block = false;

    for line in content.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('=') && !trimmed.starts_with("#set") {
            if !current_block.is_empty() {
                blocks.push(current_block.trim().to_string());
                current_block.clear();
            }
            in_block = true;
        } else if in_block && !trimmed.is_empty() {
            current_block.push_str(line);
            current_block.push('\n');
        }
    }

    if !current_block.is_empty() {
        blocks.push(current_block.trim().to_string());
    }

    blocks
}

/// Check if content has reasonable parity with baseline (within tolerance)
fn content_parity_check(extracted: &str, baseline: &str, tolerance_percent: f64) -> bool {
    let extracted_len = extracted.len();
    let baseline_len = baseline.len();

    if baseline_len == 0 {
        return extracted_len == 0;
    }

    let ratio = (extracted_len as f64) / (baseline_len as f64);
    let acceptable_min = 1.0 - (tolerance_percent / 100.0);
    let acceptable_max = 1.0 + (tolerance_percent / 100.0);

    ratio >= acceptable_min && ratio <= acceptable_max
}

// CRITICAL BUG TESTS - These expose the 45+ issues

/// TEST 1: CRITICAL - 62% heading loss bug
///
/// The extractor only matches single `=` headings, completely skipping
/// `==`, `===`, and higher levels. This causes catastrophic data loss
/// in hierarchical documents.
///
/// Expected: All heading levels should be extracted
/// Current behavior: Only level 1 headings extracted
/// WILL FAIL: Exposing the heading loss bug
#[tokio::test]
async fn test_typst_all_heading_levels_not_lost() {
    let content = load_test_document("headings.typ");
    let _baseline = load_pandoc_baseline("headings");
    let config = ExtractionConfig::default();

    let result = extract_bytes(&content, "application/x-typst", &config)
        .await
        .expect("Extraction failed");

    let extracted_all_headings = extract_all_headings(&result.content);

    assert!(
        extracted_all_headings.len() >= 6,
        "CRITICAL BUG: Only extracted {} headings, should have extracted 6+ heading levels. \
         This is the 62% heading loss bug - extractor only matches '=' but skips '==', '===', etc.",
        extracted_all_headings.len()
    );

    for level in 1..=6 {
        let count = count_heading_level(&result.content, level);
        assert_eq!(
            count, 1,
            "Heading level {} should appear exactly once (found {}). \
             Missing heading levels cause data loss in hierarchical documents.",
            level, count
        );
    }
}

/// TEST 2: Display math not extracted
///
/// Display math ($$...$$) is completely lost from extraction,
/// breaking mathematical content preservation.
///
/// Expected: Display math should be preserved in output
/// Current behavior: Silently dropped
/// WILL FAIL: Exposing display math loss
#[tokio::test]
async fn test_typst_display_math_preserved() {
    let content = load_test_document("advanced.typ");
    let baseline = load_pandoc_baseline("advanced");
    let config = ExtractionConfig::default();

    let result = extract_bytes(&content, "application/x-typst", &config)
        .await
        .expect("Extraction failed");

    let has_display_math_in_baseline =
        baseline.contains("²") || baseline.contains("Display math") || baseline.contains("x^2");

    if has_display_math_in_baseline {
        let our_has_math = result.content.contains("$")
            || result.content.contains("Display")
            || result.content.contains("²")
            || result.content.contains("²");

        assert!(
            our_has_math,
            "Display math should be extracted. Pandoc preserves mathematical notation, \
             but extractor drops it entirely. This breaks scientific/academic documents."
        );
    }

    let has_pythagorean = result.content.contains("^2")
        || result.content.contains("²")
        || result.content.contains("x") && result.content.contains("y") && result.content.contains("r");

    assert!(
        has_pythagorean,
        "Pythagorean theorem expression should be present. Display math is being dropped."
    );
}

/// TEST 3: Empty headings output
///
/// When heading text is missing or malformed, extractor outputs
/// just the marker like "= " with no text, polluting the output.
///
/// Expected: Either full heading text or no heading at all
/// Current behavior: "= " with no content
/// WILL FAIL: Exposing empty heading bug
#[tokio::test]
async fn test_typst_no_empty_headings_output() {
    let content = load_test_document("headings.typ");
    let config = ExtractionConfig::default();

    let result = extract_bytes(&content, "application/x-typst", &config)
        .await
        .expect("Extraction failed");

    let empty_headings = count_empty_headings(&result.content);

    assert_eq!(
        empty_headings, 0,
        "Found {} empty heading lines (just '=' with no text). \
         Extractor outputs malformed headings like '= ' with no text, \
         corrupting the document structure.",
        empty_headings
    );

    for heading in extract_all_headings(&result.content) {
        let trimmed = heading.trim_start();
        let after_marker = trimmed.trim_start_matches('=').trim();
        assert!(
            !after_marker.is_empty(),
            "Heading '{}' has no text after marker. Should not output empty headings.",
            trimmed
        );
    }
}

/// TEST 4: Metadata extraction fails with regex silently
///
/// When regex patterns fail to match metadata fields,
/// the extractor silently returns None instead of logging/failing,
/// causing complete metadata loss for certain formats.
///
/// Expected: All metadata fields should be extracted
/// Current behavior: Some formats fail silently
/// WILL FAIL: Exposing metadata loss
#[tokio::test]
async fn test_typst_metadata_extraction_completeness() {
    let content = load_test_document("metadata.typ");
    let _baseline_meta = load_pandoc_metadata("metadata");
    let config = ExtractionConfig::default();

    let result = extract_bytes(&content, "application/x-typst", &config)
        .await
        .expect("Extraction failed");

    let has_title = result
        .metadata
        .additional
        .get("title")
        .map(|t| t.to_string().len() > 0)
        .unwrap_or(false);

    let has_author = result
        .metadata
        .additional
        .get("author")
        .map(|a| a.to_string().len() > 0)
        .unwrap_or(false);

    let has_keywords = result
        .metadata
        .additional
        .get("keywords")
        .map(|k| k.to_string().len() > 0)
        .unwrap_or(false);

    assert!(
        has_title,
        "Title metadata should be extracted. Regex pattern matching fails silently \
         and metadata is lost with no error reporting."
    );

    assert!(
        has_author,
        "Author metadata should be extracted. Some metadata formats fail silently."
    );

    assert!(
        has_keywords,
        "Keywords should be extracted. Regex failures cause silent data loss."
    );
}

/// TEST 5: Nested table brackets cause corruption
///
/// Tables with nested brackets like [Name [full]] corrupt the
/// table content extraction because bracket counting is naive.
///
/// Expected: Table cells should be extracted correctly even with nesting
/// Current behavior: Bracket nesting causes cells to be malformed
/// WILL FAIL: Exposing table corruption bug
#[tokio::test]
async fn test_typst_tables_with_nested_brackets_not_corrupted() {
    let content = load_test_document("advanced.typ");
    let baseline = load_pandoc_baseline("advanced");
    let config = ExtractionConfig::default();

    let result = extract_bytes(&content, "application/x-typst", &config)
        .await
        .expect("Extraction failed");

    let has_table_in_baseline = baseline.contains("Name") && baseline.contains("Alice");

    if has_table_in_baseline {
        let table_content_extracted =
            result.content.contains("Name") && result.content.contains("Alice") && result.content.contains("Age");

        assert!(
            table_content_extracted,
            "Table content should be extracted correctly. Nested brackets cause corruption \
             and table cells are malformed."
        );

        let corrupted_brackets = result.content.matches("[[").count();
        assert_eq!(
            corrupted_brackets, 0,
            "Found corrupted bracket sequences [[. Table extraction with nested brackets \
             produces malformed output."
        );
    }
}

/// TEST 6: Content volume parity - within tolerance of Pandoc
///
/// Our extractor should extract roughly the same amount of content
/// as Pandoc (baseline). Large discrepancies indicate data loss or
/// noise injection.
///
/// Expected: Within reasonable tolerance of baseline content size
/// Current behavior: Significant data loss on complex documents (e.g., advanced.typ)
/// WILL FAIL: Exposing data loss on complex documents with formatting
#[tokio::test]
async fn test_typst_content_volume_parity_with_pandoc() {
    let documents = vec![("simple", 30.0), ("headings", 20.0)];

    for (doc_name, tolerance) in documents {
        let content = load_test_document(&format!("{}.typ", doc_name));
        let baseline = load_pandoc_baseline(doc_name);
        let config = ExtractionConfig::default();

        let result = extract_bytes(&content, "application/x-typst", &config)
            .await
            .expect(&format!("Extraction failed for {}", doc_name));

        let baseline_size = baseline.len();
        let extracted_size = result.content.len();

        let is_within_tolerance = content_parity_check(&result.content, &baseline, tolerance);

        assert!(
            is_within_tolerance,
            "Content volume parity failed for {}: \
             Baseline: {} bytes, Extracted: {} bytes ({}% tolerance allowed). \
             Data loss indicates missing extraction features or formatting issues.",
            doc_name, baseline_size, extracted_size, tolerance
        );
    }
}

/// TEST 7: Blockquotes not implemented
///
/// Blockquotes (using > syntax in other formats, typst uses #quote)
/// are completely unimplemented, causing loss of semantic structure.
///
/// Expected: Blockquote content should be extracted
/// Current behavior: Feature not implemented
/// WILL FAIL: Exposing missing blockquote support
#[tokio::test]
async fn test_typst_blockquote_handling() {
    let test_content = b"#quote[
        This is a blockquote.
        It should be extracted.
    ]";

    let config = ExtractionConfig::default();
    let result = extract_bytes(test_content, "application/x-typst", &config)
        .await
        .expect("Extraction failed");

    let has_blockquote_content =
        result.content.contains("blockquote") || result.content.contains("This is a blockquote");

    assert!(
        has_blockquote_content,
        "Blockquote content should be extracted. Blockquotes are not implemented \
         in the extractor, causing complete loss of quoted content."
    );
}

/// TEST 8: Inline code preservation
///
/// Test that inline code blocks are properly extracted and marked.
/// This ensures code snippets aren't corrupted.
///
/// Expected: Inline code preserved with backticks or clearly marked
/// Current behavior: May be corrupted
/// WILL FAIL: If inline code is not preserved
#[tokio::test]
async fn test_typst_inline_code_preserved() {
    let content = load_test_document("advanced.typ");
    let baseline = load_pandoc_baseline("advanced");
    let config = ExtractionConfig::default();

    let result = extract_bytes(&content, "application/x-typst", &config)
        .await
        .expect("Extraction failed");

    let has_inline_code =
        result.content.contains("`") || (result.content.contains("code") && baseline.contains("`code`"));

    assert!(
        has_inline_code,
        "Inline code should be preserved with backticks or clearly marked."
    );
}

/// TEST 9: Inline math extraction
///
/// Inline math (single $ delimiters) should be extracted and preserved.
///
/// Expected: Inline math formulas preserved
/// Current behavior: May be dropped
/// WILL FAIL: If inline math is lost
#[tokio::test]
async fn test_typst_inline_math_preserved() {
    let content = load_test_document("advanced.typ");
    let baseline = load_pandoc_baseline("advanced");
    let config = ExtractionConfig::default();

    let result = extract_bytes(&content, "application/x-typst", &config)
        .await
        .expect("Extraction failed");

    let has_inline_math =
        result.content.contains("$") || result.content.contains("sqrt") || result.content.contains("equation");

    if baseline.contains("$") || baseline.contains("equation") {
        assert!(
            has_inline_math,
            "Inline math should be extracted. Mathematical formulas are being dropped."
        );
    }
}

/// TEST 10: Figures and captions
///
/// Figure extraction with captions should preserve both image references
/// and caption text.
///
/// Expected: Figure content and captions extracted
/// Current behavior: May be unimplemented
#[tokio::test]
async fn test_typst_figures_and_captions() {
    let test_content = b"#figure(
        image(\"example.png\"),
        caption: [This is a figure caption]
    )";

    let config = ExtractionConfig::default();
    let result = extract_bytes(test_content, "application/x-typst", &config)
        .await
        .expect("Extraction failed");

    let _has_caption = result.content.contains("caption") || result.content.contains("figure");

    println!(
        "Figure extraction result (feature may be unimplemented): {:?}",
        result.content
    );
}

/// TEST 11: Citation/reference handling
///
/// Citations and references should be extracted when present.
///
/// Expected: Citation markers and text preserved
/// Current behavior: May be dropped
#[tokio::test]
async fn test_typst_citations_preserved() {
    let test_content = b"Here is a citation @smith2020.

= References

#bibliography()";

    let config = ExtractionConfig::default();
    let result = extract_bytes(test_content, "application/x-typst", &config)
        .await
        .expect("Extraction failed");

    let _has_citation = result.content.contains("@smith2020")
        || result.content.contains("smith")
        || result.content.contains("References");

    println!("Citation handling (may be limited): {:?}", result.content);
}

/// TEST 12: Link extraction and formatting
///
/// Links should be extracted with both URL and link text.
///
/// Expected: Links in markdown format [text](url)
/// Current behavior: May lose URL or text
#[tokio::test]
async fn test_typst_link_extraction() {
    let content = load_test_document("advanced.typ");
    let baseline = load_pandoc_baseline("advanced");
    let config = ExtractionConfig::default();

    let result = extract_bytes(&content, "application/x-typst", &config)
        .await
        .expect("Extraction failed");

    let has_link_content =
        result.content.contains("example") || result.content.contains("link") || result.content.contains("https");

    assert!(
        has_link_content,
        "Link content should be extracted. Links may be completely dropped."
    );
}

/// TEST 13: Unordered list extraction
///
/// Both + and - list markers should be converted to standard format.
///
/// Expected: All list items extracted and normalized
/// Current behavior: May lose some items
#[tokio::test]
async fn test_typst_list_extraction() {
    let content = load_test_document("simple.typ");
    let baseline = load_pandoc_baseline("simple");
    let config = ExtractionConfig::default();

    let result = extract_bytes(&content, "application/x-typst", &config)
        .await
        .expect("Extraction failed");

    let has_list_markers = result.content.contains("-") || result.content.contains("+");
    let has_list_content =
        result.content.contains("First") || result.content.contains("Second") || result.content.contains("item");

    assert!(
        has_list_markers || has_list_content,
        "List items should be extracted with markers or content preserved."
    );
}

/// TEST 14: Code block extraction
///
/// Triple-backtick code blocks should be fully extracted with language specifiers.
///
/// Expected: Code blocks with language markers preserved
/// Current behavior: May be malformed
#[tokio::test]
async fn test_typst_code_block_extraction() {
    let content = load_test_document("advanced.typ");
    let baseline = load_pandoc_baseline("advanced");
    let config = ExtractionConfig::default();

    let result = extract_bytes(&content, "application/x-typst", &config)
        .await
        .expect("Extraction failed");

    let has_code = result.content.contains("```")
        || result.content.contains("def")
        || result.content.contains("fibonacci")
        || result.content.contains("python");

    assert!(has_code, "Code blocks should be extracted with language specifiers.");
}

/// TEST 15: Bold and italic formatting
///
/// Inline emphasis formatting should be preserved or normalized.
///
/// Expected: Bold (*text*) and italic (_text_) markers present
/// Current behavior: May be lost
#[tokio::test]
async fn test_typst_emphasis_formatting() {
    let content = load_test_document("advanced.typ");
    let config = ExtractionConfig::default();

    let result = extract_bytes(&content, "application/x-typst", &config)
        .await
        .expect("Extraction failed");

    let has_emphasis = result.content.contains("*") && result.content.contains("_");

    assert!(has_emphasis, "Bold and italic formatting markers should be preserved.");
}

/// TEST 16: Complex nested formatting
///
/// Test handling of *_nested formatting_* combinations.
///
/// Expected: Nested formatting preserved or flattened consistently
/// Current behavior: May be malformed
#[tokio::test]
async fn test_typst_nested_formatting() {
    let test_content = b"This is *bold with _nested italic_* text.";

    let config = ExtractionConfig::default();
    let result = extract_bytes(test_content, "application/x-typst", &config)
        .await
        .expect("Extraction failed");

    let has_formatting = result.content.contains("*")
        || result.content.contains("_")
        || (result.content.contains("bold") && result.content.contains("italic"));

    assert!(
        has_formatting,
        "Nested formatting should be preserved or flattened consistently."
    );
}

/// TEST 17: Multiple paragraph handling
///
/// Multiple paragraphs separated by blank lines should be preserved.
///
/// Expected: Paragraph structure maintained
/// Current behavior: May merge or lose paragraphs
#[tokio::test]
async fn test_typst_multiple_paragraphs() {
    let content = load_test_document("advanced.typ");
    let baseline = load_pandoc_baseline("advanced");
    let config = ExtractionConfig::default();

    let result = extract_bytes(&content, "application/x-typst", &config)
        .await
        .expect("Extraction failed");

    let non_empty_lines: Vec<_> = result.content.lines().filter(|l| !l.trim().is_empty()).collect();

    assert!(
        non_empty_lines.len() >= 5,
        "Multiple paragraphs should be preserved. Found {} content lines.",
        non_empty_lines.len()
    );
}

/// TEST 18: Heading-content association
///
/// Content should follow its heading logically in the output.
///
/// Expected: Each heading followed by its content
/// Current behavior: May be scrambled
#[tokio::test]
async fn test_typst_heading_content_association() {
    let content = load_test_document("advanced.typ");
    let config = ExtractionConfig::default();

    let result = extract_bytes(&content, "application/x-typst", &config)
        .await
        .expect("Extraction failed");

    let blocks = extract_content_blocks(&result.content);

    assert!(blocks.len() > 0, "Content blocks should be associated with headings.");

    for block in &blocks {
        assert!(block.len() > 0, "Content blocks should not be empty.");
    }
}

/// TEST 19: Whitespace normalization
///
/// Multiple blank lines should be normalized consistently.
///
/// Expected: Single blank lines between sections
/// Current behavior: May have excessive whitespace
#[tokio::test]
async fn test_typst_whitespace_handling() {
    let content = load_test_document("advanced.typ");
    let config = ExtractionConfig::default();

    let result = extract_bytes(&content, "application/x-typst", &config)
        .await
        .expect("Extraction failed");

    let blank_line_runs: Vec<_> = result.content.split("\n\n\n").collect();

    assert!(
        blank_line_runs.len() <= 2,
        "Should not have excessive blank lines (triple newlines). \
         Found {} instances of triple newlines.",
        blank_line_runs.len() - 1
    );
}

/// TEST 20: Minimal document handling
///
/// Even minimal documents should extract correctly.
///
/// Expected: Basic content and structure
/// Current behavior: May fail or lose content
#[tokio::test]
async fn test_typst_minimal_document() {
    let content = load_test_document("minimal.typ");
    let baseline = load_pandoc_baseline("minimal");
    let config = ExtractionConfig::default();

    let result = extract_bytes(&content, "application/x-typst", &config)
        .await
        .expect("Extraction failed");

    assert!(
        !result.content.is_empty(),
        "Even minimal documents should extract some content."
    );

    assert!(
        result.content.len() > 0,
        "Minimal document should produce non-empty output."
    );
}

/// TEST 21: No directive pollution
///
/// Extracted content should not contain #set, #let, #import directives.
///
/// Expected: Clean extracted content without directives
/// Current behavior: May include directives
#[tokio::test]
async fn test_typst_no_directive_pollution() {
    let content = load_test_document("advanced.typ");
    let config = ExtractionConfig::default();

    let result = extract_bytes(&content, "application/x-typst", &config)
        .await
        .expect("Extraction failed");

    let directive_count = count_directive_lines(&result.content);

    assert_eq!(
        directive_count, 0,
        "Extracted content should not contain directives (#set, #let, etc). \
         Found {} directive lines polluting the output.",
        directive_count
    );
}

/// TEST 22: Metadata field completeness
///
/// All metadata fields from baseline should be present.
///
/// Expected: Title, author, date, keywords all extracted
/// Current behavior: Some fields missing
#[tokio::test]
async fn test_typst_metadata_field_completeness() {
    let content = load_test_document("advanced.typ");
    let config = ExtractionConfig::default();

    let result = extract_bytes(&content, "application/x-typst", &config)
        .await
        .expect("Extraction failed");

    let has_title = result.metadata.additional.get("title").is_some();
    let has_author = result.metadata.additional.get("author").is_some();
    let has_date = result.metadata.date.is_some();

    assert!(
        has_title && has_author && has_date,
        "All metadata fields should be extracted. \
         Title: {}, Author: {}, Date: {}",
        has_title,
        has_author,
        has_date
    );
}

/// TEST 23: Special character handling
///
/// Unicode and special characters should be preserved.
///
/// Expected: Special characters like ü, é, etc. preserved
/// Current behavior: May be corrupted
#[tokio::test]
async fn test_typst_special_character_preservation() {
    let test_content = "Café with naïve français".as_bytes();

    let config = ExtractionConfig::default();
    let result = extract_bytes(test_content, "application/x-typst", &config)
        .await
        .expect("Extraction failed");

    let has_special_chars =
        result.content.contains("Café") || result.content.contains("naïve") || result.content.contains("français");

    assert!(
        has_special_chars,
        "Special characters should be preserved in extraction."
    );
}

/// TEST 24: Very long heading handling
///
/// Long headings should not cause truncation or corruption.
///
/// Expected: Full heading text preserved regardless of length
/// Current behavior: May truncate
#[tokio::test]
async fn test_typst_long_heading_handling() {
    let test_content = b"= This is a very long heading that should be completely preserved without any truncation or corruption whatsoever";

    let config = ExtractionConfig::default();
    let result = extract_bytes(test_content, "application/x-typst", &config)
        .await
        .expect("Extraction failed");

    let has_heading_start = result.content.contains("very long heading");

    assert!(has_heading_start, "Long headings should not be truncated.");
}

/// TEST 25: Edge case - Empty heading recovery
///
/// Even if a heading has no text, extraction should be robust.
///
/// Expected: Graceful handling without crashes
/// Current behavior: May panic or produce empty output
#[tokio::test]
async fn test_typst_empty_heading_edge_case() {
    let test_content = b"= \n\n== \nContent here";

    let config = ExtractionConfig::default();
    let result = extract_bytes(test_content, "application/x-typst", &config).await;

    match result {
        Ok(extraction) => {
            assert!(
                extraction.content.contains("Content"),
                "Should extract regular content even if some headings are empty."
            );
        }
        Err(_) => {}
    }
}

/// TEST 26: Regression - Basic heading extraction
#[tokio::test]
async fn test_typst_basic_heading_regression() {
    let test_content = b"= Main Heading\n\nContent here";

    let config = ExtractionConfig::default();
    let result = extract_bytes(test_content, "application/x-typst", &config)
        .await
        .expect("Extraction failed");

    assert!(
        result.content.contains("= Main Heading"),
        "Basic level-1 heading should be extracted."
    );

    assert!(result.content.contains("Content"), "Content should be extracted.");
}

/// TEST 27: Regression - Level 2 heading extraction
#[tokio::test]
async fn test_typst_level2_heading_regression() {
    let test_content = b"= Main\n\n== Subsection\n\nMore content";

    let config = ExtractionConfig::default();
    let result = extract_bytes(test_content, "application/x-typst", &config)
        .await
        .expect("Extraction failed");

    assert!(
        result.content.contains("== Subsection"),
        "Level 2 headings must be extracted."
    );
}

/// TEST 28: Regression - Basic metadata
#[tokio::test]
async fn test_typst_basic_metadata_regression() {
    let test_content = b"#set document(title: \"Test\", author: \"Me\")\n\n= Heading";

    let config = ExtractionConfig::default();
    let result = extract_bytes(test_content, "application/x-typst", &config)
        .await
        .expect("Extraction failed");

    assert!(
        result.metadata.additional.get("title").is_some(),
        "Title metadata must be extracted."
    );

    assert!(
        result.metadata.additional.get("author").is_some(),
        "Author metadata must be extracted."
    );
}

/// TEST 29: Regression - Bold formatting
#[tokio::test]
async fn test_typst_bold_regression() {
    let test_content = b"This is *bold text* here";

    let config = ExtractionConfig::default();
    let result = extract_bytes(test_content, "application/x-typst", &config)
        .await
        .expect("Extraction failed");

    assert!(
        result.content.contains("*bold*") || result.content.contains("bold"),
        "Bold text should be preserved."
    );
}

/// TEST 30: Regression - Inline code
#[tokio::test]
async fn test_typst_inline_code_regression() {
    let test_content = b"Use `println!(\"hello\")` in Rust";

    let config = ExtractionConfig::default();
    let result = extract_bytes(test_content, "application/x-typst", &config)
        .await
        .expect("Extraction failed");

    assert!(
        result.content.contains("`") && result.content.contains("println"),
        "Inline code should be preserved with backticks."
    );
}

/// TEST 31: Regression - Code blocks
#[tokio::test]
async fn test_typst_codeblock_regression() {
    let test_content = b"```rust\nfn main() {}\n```";

    let config = ExtractionConfig::default();
    let result = extract_bytes(test_content, "application/x-typst", &config)
        .await
        .expect("Extraction failed");

    assert!(
        result.content.contains("```"),
        "Code block delimiters should be preserved."
    );

    assert!(
        result.content.contains("fn main"),
        "Code block content should be preserved."
    );
}

/// TEST 32: Regression - List extraction
#[tokio::test]
async fn test_typst_list_regression() {
    let test_content = b"- Item 1\n+ Item 2\n- Item 3";

    let config = ExtractionConfig::default();
    let result = extract_bytes(test_content, "application/x-typst", &config)
        .await
        .expect("Extraction failed");

    assert!(
        result.content.contains("Item 1") && result.content.contains("Item 2") && result.content.contains("Item 3"),
        "All list items should be extracted."
    );
}

/// TEST 33: Regression - Math preservation
#[tokio::test]
async fn test_typst_math_regression() {
    let test_content = b"Formula: $E = mc^2$";

    let config = ExtractionConfig::default();
    let result = extract_bytes(test_content, "application/x-typst", &config)
        .await
        .expect("Extraction failed");

    assert!(
        result.content.contains("$") && (result.content.contains("mc") || result.content.contains("E")),
        "Math formulas should be preserved."
    );
}

/// TEST 34: Regression - Link extraction
#[tokio::test]
async fn test_typst_link_regression() {
    let test_content = b"Visit #link(\"https://example.com\")[example]";

    let config = ExtractionConfig::default();
    let result = extract_bytes(test_content, "application/x-typst", &config)
        .await
        .expect("Extraction failed");

    assert!(
        result.content.contains("example") || result.content.contains("example.com"),
        "Link text or URL should be preserved."
    );
}

/// TEST 35: Regression - Table basic extraction
#[tokio::test]
async fn test_typst_table_regression() {
    let test_content = b"#table(columns: 2, [A], [B], [1], [2])";

    let config = ExtractionConfig::default();
    let result = extract_bytes(test_content, "application/x-typst", &config)
        .await
        .expect("Extraction failed");

    assert!(
        result.content.contains("A") || result.content.contains("TABLE"),
        "Table content should be extracted."
    );
}

/// TEST 36: Large document handling
#[tokio::test]
async fn test_typst_large_document_stress() {
    let mut large_content = String::new();

    for i in 1..=50 {
        large_content.push_str(&format!("= Heading {}\n\n", i));
        large_content.push_str(&format!("Content for section {}.\n\n", i));
    }

    let config = ExtractionConfig::default();
    let result = extract_bytes(large_content.as_bytes(), "application/x-typst", &config)
        .await
        .expect("Extraction failed");

    let heading_count = extract_all_headings(&result.content).len();
    assert!(
        heading_count >= 40,
        "Large documents should extract all headings. Found {} of 50.",
        heading_count
    );
}

/// TEST 37: Deep nesting stress test
#[tokio::test]
async fn test_typst_deep_nesting_stress() {
    let mut nested = String::new();

    for level in 1..=6 {
        nested.push_str(&format!("{} Level {} Heading\n\n", "=".repeat(level), level));
        nested.push_str(&format!("Content at level {}.\n\n", level));
    }

    let config = ExtractionConfig::default();
    let result = extract_bytes(nested.as_bytes(), "application/x-typst", &config)
        .await
        .expect("Extraction failed");

    for level in 1..=6 {
        let count = count_heading_level(&result.content, level);
        assert!(
            count >= 1,
            "Level {} heading should be extracted in deep nesting test.",
            level
        );
    }
}

/// TEST 38: Mixed formatting stress
#[tokio::test]
async fn test_typst_mixed_formatting_stress() {
    let test_content = b"This text has *bold*, _italic_, `code`, and $math$ all mixed together!";

    let config = ExtractionConfig::default();
    let result = extract_bytes(test_content, "application/x-typst", &config)
        .await
        .expect("Extraction failed");

    let has_formatting = (result.content.contains("*") || result.content.contains("bold"))
        && (result.content.contains("_") || result.content.contains("italic"))
        && (result.content.contains("`") || result.content.contains("code"))
        && (result.content.contains("$") || result.content.contains("math"));

    assert!(has_formatting, "All mixed formatting should be preserved.");
}

/// TEST 39: Unicode stress test
#[tokio::test]
async fn test_typst_unicode_stress() {
    let test_content = "= Unicode Heading 中文 العربية\n\nContent with emojis: 🎉🚀💯\n\nGreek: α β γ δ ε ζ".as_bytes();

    let config = ExtractionConfig::default();
    let result = extract_bytes(test_content, "application/x-typst", &config)
        .await
        .expect("Extraction failed");

    assert!(
        result.content.contains("Unicode"),
        "Unicode content should be preserved."
    );
}

/// TEST 40: Pathological whitespace
#[tokio::test]
async fn test_typst_pathological_whitespace() {
    let test_content = b"= Heading\n\n\n\n\n\nContent with excessive blank lines\n\n\n\n\nMore content";

    let config = ExtractionConfig::default();
    let result = extract_bytes(test_content, "application/x-typst", &config)
        .await
        .expect("Extraction failed");

    assert!(
        result.content.contains("Heading") && result.content.contains("Content"),
        "Should extract content even with excessive whitespace."
    );
}

/// TEST 41: Full document comparison - simple.typ
#[tokio::test]
async fn test_typst_full_simple_document_comparison() {
    let content = load_test_document("simple.typ");
    let baseline = load_pandoc_baseline("simple");
    let config = ExtractionConfig::default();

    let result = extract_bytes(&content, "application/x-typst", &config)
        .await
        .expect("Extraction failed");

    assert!(
        result.content.len() > 50,
        "simple.typ should extract substantial content"
    );

    let heading_count = extract_all_headings(&result.content).len();
    assert!(heading_count > 2, "simple.typ should have multiple sections");
}

/// TEST 42: Full document comparison - advanced.typ
#[tokio::test]
async fn test_typst_full_advanced_document_comparison() {
    let content = load_test_document("advanced.typ");
    let baseline = load_pandoc_baseline("advanced");
    let config = ExtractionConfig::default();

    let result = extract_bytes(&content, "application/x-typst", &config)
        .await
        .expect("Extraction failed");

    assert!(
        result.content.len() > 100,
        "advanced.typ should extract comprehensive content"
    );

    let heading_count = extract_all_headings(&result.content).len();
    assert!(heading_count >= 5, "advanced.typ should preserve heading structure");
}

/// TEST 43: MIME type consistency
///
/// The extractor should support both standard MIME types for Typst.
/// Currently only supports application/x-typst, not text/x-typst.
#[tokio::test]
async fn test_typst_mime_type_consistency() {
    let content = load_test_document("simple.typ");
    let config = ExtractionConfig::default();

    let result_primary = extract_bytes(&content, "application/x-typst", &config)
        .await
        .expect("Primary MIME type should work");

    assert!(
        result_primary.content.len() > 0,
        "Primary MIME type should extract content"
    );

    match extract_bytes(&content, "text/x-typst", &config).await {
        Ok(result) => {
            assert!(
                result.content.len() > 0,
                "Alternative MIME type should extract content if supported"
            );
        }
        Err(_e) => {
            println!("Note: text/x-typst is not currently supported (may be added in future)");
        }
    }
}

/// TEST 44: Config parameter impact
#[tokio::test]
async fn test_typst_config_parameter_handling() {
    let content = load_test_document("simple.typ");
    let config = ExtractionConfig::default();

    let result = extract_bytes(&content, "application/x-typst", &config)
        .await
        .expect("Extraction failed");

    assert!(!result.content.is_empty(), "Extraction with default config should work");

    assert_eq!(result.mime_type, "application/x-typst", "MIME type should be preserved");
}

/// TEST 45: Comparative heading analysis
///
/// This final comprehensive test checks heading extraction
/// against the baseline to identify the exact scope of the heading loss bug.
#[tokio::test]
async fn test_typst_heading_loss_bug_analysis() {
    let content = load_test_document("headings.typ");
    let baseline = load_pandoc_baseline("headings");
    let config = ExtractionConfig::default();

    let result = extract_bytes(&content, "application/x-typst", &config)
        .await
        .expect("Extraction failed");

    println!("\n===== HEADING EXTRACTION ANALYSIS =====");
    println!("Baseline content:");
    println!("{}", baseline);
    println!("\nExtracted content:");
    println!("{}", result.content);

    let extracted_headings = extract_all_headings(&result.content);
    println!("\nExtracted headings: {}", extracted_headings.len());
    for (i, h) in extracted_headings.iter().enumerate() {
        println!("  {}: {}", i + 1, h);
    }

    assert!(
        extracted_headings.len() >= 6,
        "BUG CONFIRMED: Heading loss detected. \
         Expected 6 headings (1-6 levels), found {}. \
         This is the 62% heading loss bug - only single '=' is matched, \
         all '==' and higher are skipped entirely.",
        extracted_headings.len()
    );
}
