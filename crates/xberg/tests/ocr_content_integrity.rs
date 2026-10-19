//! Regression test for https://github.com/xberg-io/xberg/issues/706
//!
//! Tesseract OCR was producing corrupted page content: the top-level `content`
//! field contained the coherent HOCR-rendered text followed by a word-by-word
//! dump of every OcrText element, effectively doubling the output.
//!
//! Root cause: `inject_ocr_elements_from_vec` pushed each OcrElement into
//! `InternalDocument::elements` as an `ElementKind::OcrText`. The rendering
//! pipeline (`render_plain`) iterated those elements and appended every word
//! token back into `content`, on top of the already-rendered HOCR string.
//!
//! Fix: OCR elements are now stored directly in `InternalDocument::prebuilt_ocr_elements`
//! (bypassing the rendering pipeline) and page content is set via
//! `InternalDocument::prebuilt_pages` (bypassing the word-grouped fallback in
//! `build_pages`).

#![cfg(feature = "ocr")]

mod helpers;
use helpers::extract_uri_document_blocking;

use helpers::*;
use xberg::core::config::{ExtractionConfig, OcrConfig, PageConfig};
use xberg::types::TesseractConfig;

/// Content must not be doubled when OCR is enabled.
///
/// Before the fix, `content` contained the HOCR-rendered paragraph text
/// immediately followed by a word-by-word dump of every OcrText element,
/// roughly doubling the word count.  After the fix the two representations
/// must be absent: `content` should equal approximately what is in `pages[0].content`.
#[test]
fn test_ocr_content_not_doubled() {
    if skip_if_missing("images/test_hello_world.png") {
        return;
    }

    let file_path = get_test_file_path("images/test_hello_world.png");
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: vec!["eng".to_string()],
            ..Default::default()
        }),
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        use_cache: false,
        ..Default::default()
    };

    let result = extract_uri_document_blocking(&file_path, None, &config).expect("OCR extraction must succeed");

    let content_words: Vec<&str> = result.content.split_whitespace().collect();

    if content_words.is_empty() {
        return;
    }

    let pages = result
        .pages
        .as_ref()
        .expect("pages must be populated when extract_pages=true");
    assert!(!pages.is_empty(), "at least one page must be present");

    let page_content = &pages[0].content;
    let page_words: Vec<&str> = page_content.split_whitespace().collect();

    if !page_words.is_empty() {
        let ratio = content_words.len() as f64 / page_words.len() as f64;
        assert!(
            ratio <= 1.3,
            "content word count ({}) is more than 30% larger than pages[0].content word count ({}). \
             This indicates doubled output — word-token dump appended after HOCR text (issue #706). \
             ratio = {:.2}",
            content_words.len(),
            page_words.len(),
            ratio,
        );
    }

    if page_content.trim().len() > 4 {
        let trimmed = page_content.trim();
        let doubled = format!("{trimmed}{trimmed}");
        assert!(
            !result.content.contains(doubled.as_str()),
            "content appears to contain page text concatenated with itself — doubled output (issue #706)",
        );
    }
}

/// Page content must match the top-level content (after trimming) when there
/// is only one page, for any image that produces non-empty OCR output.
#[test]
fn test_ocr_page_content_matches_top_level_content() {
    if skip_if_missing("images/ocr_image.jpg") {
        return;
    }

    let file_path = get_test_file_path("images/ocr_image.jpg");
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: vec!["eng".to_string()],
            ..Default::default()
        }),
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        use_cache: false,
        ..Default::default()
    };

    let result = extract_uri_document_blocking(&file_path, None, &config).expect("OCR extraction must succeed");

    if result.content.trim().is_empty() {
        return;
    }

    let pages = result
        .pages
        .as_ref()
        .expect("pages must be populated when extract_pages=true");
    assert!(!pages.is_empty(), "at least one page must be present");

    let top_words = result.content.split_whitespace().count();
    let page_words = pages[0].content.split_whitespace().count();

    if top_words > 0 && page_words > 0 {
        let ratio = top_words as f64 / page_words.max(1) as f64;
        assert!(
            ratio <= 1.3,
            "top-level content ({} words) is more than 30% larger than pages[0].content ({} words). \
             Indicates word-dump appended to top-level content but missing from page — issue #706.",
            top_words,
            page_words,
        );
    }
}

/// A detected table's text must not also survive as ordinary paragraph text (issue #1571).
///
/// `hocr_document` is parsed from the raw hOCR before table detection runs, so nothing
/// removes a table's words from it once `tables` is computed. Every consumer built from
/// `internal_document` therefore carries the table's words twice: once as `tables[].markdown`
/// and once as prose. Regression coverage for #706 above uses a table-free image and never
/// exercises this path, so this test uses `images/simple_table.png` with table detection on.
#[test]
fn test_ocr_table_text_not_duplicated_in_content() {
    if skip_if_missing("images/simple_table.png") {
        return;
    }

    let file_path = get_test_file_path("images/simple_table.png");
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: vec!["eng".to_string()],
            tesseract_config: Some(TesseractConfig {
                enable_table_detection: true,
                table_min_confidence: 0.0,
                // The default 50px column threshold is too tight for this fixture's font and
                // splits it into extra spurious columns that fail `post_process_table`'s
                // well-formedness check, so table detection never fires and the test can't
                // reproduce #1571. 80px merges those back into the real 4 columns.
                table_column_threshold: 80,
                table_row_threshold_ratio: 0.5,
                ..Default::default()
            }),
            ..Default::default()
        }),
        force_ocr: false,
        use_cache: false,
        ..Default::default()
    };

    let result = extract_uri_document_blocking(&file_path, None, &config).expect("OCR extraction must succeed");

    assert!(
        !result.tables.is_empty(),
        "table detection must fire on this fixture for the test to exercise the #1571 path"
    );

    // Pick a distinctive cell value from the detected table and confirm it appears in
    // `doc.content` only as many times as it appears across the table cells themselves
    // (usually once) -- not once more as a duplicated paragraph outside the table.
    let table = &result.tables[0];
    let mut cell_values: Vec<&str> = table
        .cells
        .iter()
        .flatten()
        .map(String::as_str)
        .filter(|cell| cell.trim().len() >= 3)
        .collect();
    cell_values.sort_unstable();
    cell_values.dedup();

    for cell in cell_values {
        let occurrences_in_table_cells = table.cells.iter().flatten().filter(|c| c.as_str() == cell).count();
        let occurrences_in_content = result.content.matches(cell).count();
        assert!(
            occurrences_in_content <= occurrences_in_table_cells,
            "table cell {cell:?} appears {occurrences_in_content} times in doc.content but only \
             {occurrences_in_table_cells} times among table cells -- its text is duplicated outside \
             the table (issue #1571)",
        );
    }
}

/// A blank PDF page whose text is recovered from its embedded images keeps that text when the
/// retry also finds a table (#1571), and prints the table once (#2014). The fixture draws corpus
/// objects `5883a36a12` and `0c1a8b71c1` under a zero-area clip, so the page renders blank and
/// only the embedded-image retry reads it. `Banana` appears only in the table. ~keep
#[cfg(feature = "pdf")]
#[test]
fn test_ocr_embedded_image_retry_text_survives_a_retry_table() {
    use xberg::core::config::OutputFormat;

    const EXPECTED_PLAIN: &str = r#"wee oe aoa

301016

Ko] te

10.000

0.002

~

TOTAL IS

bd dv

rAX

455

Subtotal

0, 0u0u

(atv

re

00

60.000

TOTAL

0.000

Cee wwe we ee me wane e

enerqnco mech)

Product Price Quantity Total
Apple 50 10 00
Banana 20 15 00
Orange 00 8 00
Total   00"#;
    const EXPECTED_MARKDOWN: &str = r#"wee oe aoa

301016

Ko] te

10.000

0.002

\~

TOTAL IS

bd dv

rAX

455

Subtotal

0, 0u0u

(atv

re

00

60.000

TOTAL

0.000

Cee wwe we ee me wane e

enerqnco mech)

| Product | Price | Quantity | Total |
| --- | --- | --- | --- |
| Apple | 50 | 10 | 00 |
| Banana | 20 | 15 | 00 |
| Orange | 00 | 8 | 00 |
| Total |  |  | 00 |
"#;
    let file_path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/ocr/clipped_table_and_receipt.pdf");
    for output_format in [OutputFormat::Plain, OutputFormat::Markdown] {
        let config = ExtractionConfig {
            ocr: Some(OcrConfig {
                backend: "tesseract".to_string(),
                language: vec!["eng".to_string()],
                tesseract_config: Some(TesseractConfig {
                    enable_table_detection: true,
                    use_cache: false,
                    ..Default::default()
                }),
                ..Default::default()
            }),
            force_ocr: true,
            output_format: output_format.clone(),
            use_cache: false,
            ..Default::default()
        };

        let result = extract_uri_document_blocking(&file_path, None, &config).expect("OCR extraction must succeed");

        assert_eq!(result.tables.len(), 1, "the retry must find exactly one table");
        let expected = match output_format {
            OutputFormat::Plain => EXPECTED_PLAIN,
            OutputFormat::Markdown => EXPECTED_MARKDOWN,
            _ => unreachable!("the regression covers only Plain and Markdown"),
        };
        assert_eq!(result.content, expected);
    }
}

/// A blank PDF page whose only embedded image is a table keeps that table, printed once, when
/// the table holds every line the retry recovered (#2014). With PaddleOCR compiled in, the default
/// Tesseract setup runs as a pipeline that falls back to PaddleOCR when a page scores as empty, so
/// the recovered table text must count as page text. The fixture draws corpus object `5883a36a12`
/// under a zero-area clip, so only the embedded-image retry reads it. ~keep
#[cfg(feature = "pdf")]
#[test]
fn test_ocr_embedded_image_retry_keeps_a_table_that_holds_all_of_its_text() {
    use xberg::core::config::OutputFormat;

    const EXPECTED_PLAIN: &str = r#"Product Price Quantity Total
Apple 50 10 00
Banana 20 15 00
Orange 00 8 00
Total   00"#;
    const EXPECTED_MARKDOWN: &str = r#"| Product | Price | Quantity | Total |
| --- | --- | --- | --- |
| Apple | 50 | 10 | 00 |
| Banana | 20 | 15 | 00 |
| Orange | 00 | 8 | 00 |
| Total |  |  | 00 |
"#;
    let file_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/ocr/clipped_table.pdf");
    for output_format in [OutputFormat::Plain, OutputFormat::Markdown] {
        let config = ExtractionConfig {
            ocr: Some(OcrConfig {
                backend: "tesseract".to_string(),
                language: vec!["eng".to_string()],
                tesseract_config: Some(TesseractConfig {
                    enable_table_detection: true,
                    use_cache: false,
                    ..Default::default()
                }),
                ..Default::default()
            }),
            force_ocr: true,
            output_format: output_format.clone(),
            use_cache: false,
            ..Default::default()
        };

        let result = extract_uri_document_blocking(&file_path, None, &config).expect("OCR extraction must succeed");

        assert_eq!(result.tables.len(), 1, "the retry must keep exactly one table");
        let expected = match output_format {
            OutputFormat::Plain => EXPECTED_PLAIN,
            OutputFormat::Markdown => EXPECTED_MARKDOWN,
            _ => unreachable!("the regression covers only Plain and Markdown"),
        };
        assert_eq!(result.content, expected);
    }
}
