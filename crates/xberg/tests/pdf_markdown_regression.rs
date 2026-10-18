//! PDF extraction regression tests using ground truth.
//!
//! These tests ensure extraction quality does not regress across all output formats
//! (Markdown, Djot, Plain) by comparing extracted text against ground truth files
//! using word-level F1 scoring.
//!
//! Two extraction routes are tested:
//! - **PDF (native)**: Direct text extraction from searchable PDFs
//! - **OCR**: Image rendering → Tesseract OCR → plain text
//!
//! All quality gates run by default:
//!   cargo test -p xberg --features "pdf,ocr" --test pdf_markdown_regression -- --nocapture
//!
//! The OCR gate needs Tesseract plus the `eng` traineddata on `TESSDATA_PREFIX`;
//! `scripts/ci/rust/run-unit-tests.sh` installs both before invoking cargo.
//!
//! All gates need `test_documents/`, which is bucket-fetched and absent from a bare
//! checkout — run `python3 test_documents/scripts/fetch_corpus.py`.

#![allow(clippy::print_stdout, clippy::print_stderr, clippy::dbg_macro)] // ~keep: test/bench binaries print by design; org logging policy exempts tests
#![cfg(feature = "pdf")]

#[path = "pdf_markdown_regression/ground_truth.rs"]
mod ground_truth;
mod helpers;
use ground_truth::*;
use helpers::extract_uri_document_blocking;

use helpers::*;
use std::collections::HashMap;
use std::path::PathBuf;
use xberg::core::config::{ExtractionConfig, OutputFormat};

/// Tokenize text into normalized lowercase words for comparison.
fn tokenize(text: &str) -> Vec<String> {
    text.split_whitespace()
        .map(|w| w.trim_matches(|c: char| c.is_ascii_punctuation()).to_lowercase())
        .filter(|w| !w.is_empty())
        .collect()
}

/// Compute word-level bag-of-words precision, recall, and F1 between extracted and ground truth.
fn word_f1(extracted: &str, ground_truth: &str) -> (f64, f64, f64) {
    let ext_tokens = tokenize(extracted);
    let gt_tokens = tokenize(ground_truth);

    if gt_tokens.is_empty() && ext_tokens.is_empty() {
        return (1.0, 1.0, 1.0);
    }
    if gt_tokens.is_empty() || ext_tokens.is_empty() {
        return (0.0, 0.0, 0.0);
    }

    let mut gt_bag: HashMap<&str, usize> = HashMap::new();
    for t in &gt_tokens {
        *gt_bag.entry(t.as_str()).or_insert(0) += 1;
    }

    let mut ext_bag: HashMap<&str, usize> = HashMap::new();
    for t in &ext_tokens {
        *ext_bag.entry(t.as_str()).or_insert(0) += 1;
    }

    let mut matching = 0usize;
    for (word, &ext_count) in &ext_bag {
        if let Some(&gt_count) = gt_bag.get(word) {
            matching += ext_count.min(gt_count);
        }
    }

    let precision = matching as f64 / ext_tokens.len() as f64;
    let recall = matching as f64 / gt_tokens.len() as f64;
    let f1 = if precision + recall > 0.0 {
        2.0 * precision * recall / (precision + recall)
    } else {
        0.0
    };

    (precision, recall, f1)
}

/// Resolve a ground truth name to its actual PDF file path.
fn resolve_pdf_path(gt_name: &str) -> Option<PathBuf> {
    let base = get_test_documents_dir();
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let qb_root = workspace_root.parent().unwrap().join("quality-benchmarks");

    let candidates = [
        base.join(format!("pdf/{}.pdf", gt_name)),
        base.join(format!("vendored/docling/pdf/{}.pdf", gt_name)),
        base.join(format!("vendored/pdfplumber/pdf/{}.pdf", gt_name)),
        base.join(format!("vendored/pdfplumber/pdf/from-oss-fuzz/load/{}.pdf", gt_name)),
        base.join(format!("vendored/markitdown/pdf/{}.pdf", gt_name)),
        base.join(format!("vendored/markitdown/{}.pdf", gt_name)),
        qb_root.join(format!("data/nougat/{}.pdf", gt_name)),
        qb_root.join(format!("data/pdfa/{}.pdf", gt_name)),
    ];

    candidates.into_iter().find(|p| p.exists())
}

/// Load ground truth text for a given name.
fn load_ground_truth(gt_name: &str) -> Option<String> {
    let gt_path = get_test_file_path(&format!("ground_truth/pdf/{}.txt", gt_name));
    if gt_path.exists() {
        std::fs::read_to_string(&gt_path).ok()
    } else {
        None
    }
}

/// Extract a PDF with the given output format.
fn extract_with_format(pdf_path: &std::path::Path, format: OutputFormat) -> Option<xberg::types::ExtractedDocument> {
    let config = ExtractionConfig {
        output_format: format,
        use_cache: false,
        ..Default::default()
    };
    extract_uri_document_blocking(pdf_path, None, &config).ok()
}

#[test]
fn test_hello_structure_preserves_repeated_multi_page_headings() {
    if !test_documents_available() {
        println!("Skipping: test_documents not available");
        return;
    }

    let pdf_path = resolve_pdf_path("hello_structure").expect("hello_structure.pdf fixture must exist");
    let result = extract_with_format(&pdf_path, OutputFormat::Markdown).expect("hello_structure.pdf must extract");
    assert!(
        result.content.lines().any(|line| line == "## Hello World"),
        "first repeated 24pt peer tier must remain an H2 heading; got:\n{}",
        result.content
    );
    assert!(
        result.content.lines().any(|line| line == "## Goodbye Cruel World..."),
        "subsequent repeated 24pt tiers must be H2 headings; got:\n{}",
        result.content
    );
    assert!(
        !result
            .content
            .lines()
            .any(|line| line.starts_with('#') && line.contains("I'll be back shortly")),
        "12pt body tier must remain a paragraph; got:\n{}",
        result.content
    );
}

/// Result of running the quality gate across all documents.
#[allow(dead_code)]
struct QualityGateResult {
    tested: usize,
    passed: usize,
    failed: usize,
    skipped: usize,
    avg_f1: f64,
    failures: Vec<String>,
}

/// Run the quality gate for a given output format with per-document F1 thresholds.
///
/// `threshold_scale` scales the base thresholds (e.g. 0.9 for plain text which may
/// score slightly lower due to missing formatting structure in the ground truth).
fn run_quality_gate(
    format: OutputFormat,
    ground_truth: &[(&str, f64)],
    label: &str,
    threshold_scale: f64,
) -> QualityGateResult {
    let mut tested = 0usize;
    let mut skipped = 0usize;
    let mut passed = 0usize;
    let mut failed = 0usize;
    let mut f1_sum = 0.0f64;
    let mut failures: Vec<String> = Vec::new();

    println!("\n{}", "=".repeat(100));
    println!("{} — Ground Truth Quality Gate", label);
    println!("{}", "=".repeat(100));
    println!(
        "{:<50} {:>8} {:>8} {:>8} {:>6} {:>8}",
        "Document", "Prec", "Recall", "F1", "Thresh", "Status"
    );
    println!("{}", "-".repeat(100));

    for &(gt_name, base_min_f1) in ground_truth {
        if PDFIUM_KNOWN_REGRESSIONS.contains(&gt_name) {
            println!(
                "{:<50} {:>8} {:>8} {:>8} {:>6} {:>8}",
                gt_name, "-", "-", "-", "-", "KNOWN"
            );
            skipped += 1;
            continue;
        }

        let gt = match load_ground_truth(gt_name) {
            Some(gt) => gt,
            None => {
                skipped += 1;
                continue;
            }
        };

        let pdf_path = match resolve_pdf_path(gt_name) {
            Some(p) => p,
            None => {
                skipped += 1;
                continue;
            }
        };

        let result = match extract_with_format(&pdf_path, format.clone()) {
            Some(r) => r,
            None => {
                println!(
                    "{:<50} {:>8} {:>8} {:>8} {:>6} {:>8}",
                    gt_name, "-", "-", "-", "-", "ERR"
                );
                failed += 1;
                failures.push(format!("{}: extraction failed", gt_name));
                continue;
            }
        };

        let min_f1 = base_min_f1 * threshold_scale;
        let (precision, recall, f1) = word_f1(&result.content, &gt);
        tested += 1;
        f1_sum += f1;

        let status = if f1 >= min_f1 { "PASS" } else { "FAIL" };
        if f1 < min_f1 {
            failed += 1;
            failures.push(format!("{}: F1={:.3} < threshold {:.2}", gt_name, f1, min_f1));
        } else {
            passed += 1;
        }

        println!(
            "{:<50} {:>7.1}% {:>7.1}% {:>7.1}% {:>5.0}% {:>8}",
            gt_name,
            precision * 100.0,
            recall * 100.0,
            f1 * 100.0,
            min_f1 * 100.0,
            status
        );
    }

    let avg_f1 = if tested > 0 { f1_sum / tested as f64 } else { 0.0 };

    println!("{}", "-".repeat(100));
    println!(
        "Summary: {} tested, {} passed, {} failed, {} skipped, avg F1={:.1}%",
        tested,
        passed,
        failed,
        skipped,
        avg_f1 * 100.0
    );

    if !failures.is_empty() {
        println!("\nFailures:");
        for f in &failures {
            println!("  - {}", f);
        }
    }

    QualityGateResult {
        tested,
        passed,
        failed,
        skipped,
        avg_f1,
        failures,
    }
}

#[test]
fn test_pdf_quality_gate() {
    if !test_documents_available() {
        println!("Skipping: test_documents not available");
        return;
    }

    let result = run_quality_gate(
        OutputFormat::Markdown,
        PDFIUM_GROUND_TRUTH,
        "PDFium Markdown Extraction",
        1.0,
    );

    assert!(
        result.failures.is_empty(),
        "{} document(s) fell below their F1 threshold",
        result.failures.len()
    );
    assert!(
        result.avg_f1 >= 0.78,
        "Average F1 ({:.1}%) is below 78% threshold",
        result.avg_f1 * 100.0
    );
}

#[test]
fn test_pdf_djot_quality_gate() {
    if !test_documents_available() {
        println!("Skipping: test_documents not available");
        return;
    }

    let result = run_quality_gate(OutputFormat::Djot, PDFIUM_GROUND_TRUTH, "PDFium Djot Extraction", 1.0);

    assert!(
        result.failures.is_empty(),
        "{} document(s) fell below their Djot F1 threshold",
        result.failures.len()
    );
    assert!(
        result.avg_f1 >= 0.78,
        "Average Djot F1 ({:.1}%) is below 78% threshold",
        result.avg_f1 * 100.0
    );
}

#[test]
fn test_pdf_plain_quality_gate() {
    if !test_documents_available() {
        println!("Skipping: test_documents not available");
        return;
    }

    let result = run_quality_gate(
        OutputFormat::Plain,
        PDFIUM_GROUND_TRUTH,
        "PDFium Plain Text Extraction",
        0.90,
    );

    assert!(
        result.failures.is_empty(),
        "{} document(s) fell below their Plain F1 threshold",
        result.failures.len()
    );
    assert!(
        result.avg_f1 >= 0.70,
        "Average Plain F1 ({:.1}%) is below 70% threshold",
        result.avg_f1 * 100.0
    );
}

/// Run docling.pdf parity check for a given format.
fn run_docling_parity(format: OutputFormat, label: &str, min_f1: f64) {
    let pdf_path = get_test_file_path("pdf/docling.pdf");
    if !pdf_path.exists() {
        println!("Skipping: docling.pdf not found");
        return;
    }

    let gt_path = get_test_file_path("ground_truth/docling-docling.md");
    if !gt_path.exists() {
        println!("Skipping: docling-docling.md ground truth not found");
        return;
    }

    let gt = std::fs::read_to_string(&gt_path).expect("should read docling ground truth");
    let result = extract_with_format(&pdf_path, format).expect("should extract docling.pdf");

    let (precision, recall, f1) = word_f1(&result.content, &gt);

    println!("=== docling.pdf {} parity check ===", label);
    println!(
        "  Precision: {:.1}%  Recall: {:.1}%  F1: {:.1}%",
        precision * 100.0,
        recall * 100.0,
        f1 * 100.0
    );
    println!("  Extracted words: {}", tokenize(&result.content).len());
    println!("  GT words: {}", tokenize(&gt).len());

    assert!(
        f1 >= min_f1,
        "docling.pdf {} F1 ({:.1}%) regressed below {:.0}% threshold",
        label,
        f1 * 100.0,
        min_f1 * 100.0
    );
}

#[test]
fn test_docling_pdf_parity() {
    run_docling_parity(OutputFormat::Markdown, "Markdown", 0.75);
}

#[test]
fn test_docling_pdf_djot_parity() {
    run_docling_parity(OutputFormat::Djot, "Djot", 0.75);
}

#[test]
fn test_docling_pdf_plain_parity() {
    run_docling_parity(OutputFormat::Plain, "Plain", 0.60);
}

/// Extract text via the OCR (forced) path.
#[cfg(feature = "ocr")]
fn extract_ocr(pdf_path: &std::path::Path) -> Option<xberg::types::ExtractedDocument> {
    use xberg::core::config::OcrConfig;

    let config = ExtractionConfig {
        output_format: OutputFormat::Plain,
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: vec!["eng".to_string()],
            ..Default::default()
        }),
        force_ocr: true,
        // The OCR cache is keyed on image bytes + OCR config, not on extractor code, so a
        // cached hit would let this gate pass against a stale result. ~keep
        use_cache: false,
        ..Default::default()
    };

    extract_uri_document_blocking(pdf_path, None, &config).ok()
}

/// Per-document OCR F1 quality gate.
///
/// Requires Tesseract with the `eng` traineddata reachable via `TESSDATA_PREFIX`;
/// `scripts/ci/rust/run-unit-tests.sh` provisions it before running this leg.
#[cfg(feature = "ocr")]
#[test]
fn test_ocr_quality_gate() {
    if !test_documents_available() {
        eprintln!(
            "SKIPPING test_ocr_quality_gate: test corpus not present at {} — \
             run `python3 test_documents/scripts/fetch_corpus.py`",
            get_test_documents_dir().display()
        );
        return;
    }

    let mut tested = 0usize;
    let mut skipped = 0usize;
    let mut passed = 0usize;
    let mut failed = 0usize;
    let mut f1_sum = 0.0f64;
    let mut failures: Vec<String> = Vec::new();

    println!("\n{}", "=".repeat(100));
    println!("OCR Extraction — Ground Truth Quality Gate");
    println!("{}", "=".repeat(100));
    println!(
        "{:<50} {:>8} {:>8} {:>8} {:>6} {:>8}",
        "Document", "Prec", "Recall", "F1", "Thresh", "Status"
    );
    println!("{}", "-".repeat(100));

    let mut unresolved: Vec<String> = Vec::new();

    for &(gt_name, min_f1) in OCR_GROUND_TRUTH {
        let gt = match load_ground_truth(gt_name) {
            Some(gt) => gt,
            None => {
                eprintln!(
                    "missing fixture {} — run `python3 test_documents/scripts/fetch_corpus.py`",
                    get_test_file_path(&format!("ground_truth/pdf/{gt_name}.txt")).display()
                );
                unresolved.push(format!("{gt_name} (ground truth)"));
                skipped += 1;
                continue;
            }
        };

        let pdf_path = match resolve_pdf_path(gt_name) {
            Some(p) => p,
            None => {
                eprintln!(
                    "missing fixture {gt_name}.pdf under {} — \
                     run `python3 test_documents/scripts/fetch_corpus.py`",
                    get_test_documents_dir().display()
                );
                unresolved.push(format!("{gt_name} (pdf)"));
                skipped += 1;
                continue;
            }
        };

        let result = match extract_ocr(&pdf_path) {
            Some(r) => r,
            None => {
                println!(
                    "{:<50} {:>8} {:>8} {:>8} {:>6} {:>8}",
                    gt_name, "-", "-", "-", "-", "ERR"
                );
                failed += 1;
                failures.push(format!("{}: OCR extraction failed", gt_name));
                continue;
            }
        };

        let (precision, recall, f1) = word_f1(&result.content, &gt);
        tested += 1;
        f1_sum += f1;

        let status = if f1 >= min_f1 { "PASS" } else { "FAIL" };
        if f1 < min_f1 {
            failed += 1;
            failures.push(format!("{}: F1={:.3} < threshold {:.2}", gt_name, f1, min_f1));
        } else {
            passed += 1;
        }

        println!(
            "{:<50} {:>7.1}% {:>7.1}% {:>7.1}% {:>5.0}% {:>8}",
            gt_name,
            precision * 100.0,
            recall * 100.0,
            f1 * 100.0,
            min_f1 * 100.0,
            status
        );
    }

    let avg_f1 = if tested > 0 { f1_sum / tested as f64 } else { 0.0 };

    println!("{}", "-".repeat(100));
    println!(
        "Summary: {} tested, {} passed, {} failed, {} skipped, avg F1={:.1}%",
        tested,
        passed,
        failed,
        skipped,
        avg_f1 * 100.0
    );

    if !failures.is_empty() {
        println!("\nFailures:");
        for f in &failures {
            println!("  - {}", f);
        }
    }

    assert_eq!(
        failures,
        Vec::<String>::new(),
        "{failed} document(s) failed the OCR gate (threshold miss or extraction error)"
    );
    // A fetched corpus resolves every entry. Asserting this stops the gate from
    // reporting green after silently skipping its way to zero documents.
    assert_eq!(
        tested,
        OCR_GROUND_TRUTH.len(),
        "expected all {} OCR ground-truth documents to be evaluated, only {tested} were \
         ({skipped} skipped); unresolved: {unresolved:?}",
        OCR_GROUND_TRUTH.len()
    );
    assert_eq!(
        passed, tested,
        "every evaluated document must have passed its threshold"
    );
}

/// Diagnostic dump of per-document scores, headings and previews. Asserts nothing —
/// it is a debugging aid, not coverage. Kept `#[ignore]`d so it never masquerades as
/// a passing test; the real gates are `test_pdf_quality_gate` and friends above.
#[test]
#[ignore = "diagnostic dump with no assertions; run with --ignored when investigating scores"]
fn test_pdf_detailed_snapshot() {
    if !test_documents_available() {
        eprintln!(
            "SKIPPING test_pdf_detailed_snapshot: test corpus not present at {} — \
             run `python3 test_documents/scripts/fetch_corpus.py`",
            get_test_documents_dir().display()
        );
        return;
    }

    println!("\n{}", "=".repeat(120));
    println!("PDFium Markdown — Detailed Snapshot");
    println!("{}", "=".repeat(120));

    for &(gt_name, _) in PDFIUM_GROUND_TRUTH {
        let gt = match load_ground_truth(gt_name) {
            Some(gt) => gt,
            None => continue,
        };
        let pdf_path = match resolve_pdf_path(gt_name) {
            Some(p) => p,
            None => continue,
        };
        let result = match extract_with_format(&pdf_path, OutputFormat::Markdown) {
            Some(r) => r,
            None => continue,
        };

        let (precision, recall, f1) = word_f1(&result.content, &gt);
        let ext_words = tokenize(&result.content).len();
        let gt_words = tokenize(&gt).len();
        let headings: Vec<&str> = result.content.lines().filter(|l| l.trim().starts_with('#')).collect();

        println!("\n--- {} ---", gt_name);
        println!(
            "  P={:.1}% R={:.1}% F1={:.1}%  |  extracted={} words, gt={} words  |  {} headings  |  {} tables",
            precision * 100.0,
            recall * 100.0,
            f1 * 100.0,
            ext_words,
            gt_words,
            headings.len(),
            result.tables.len()
        );

        let preview: String = result.content.chars().take(300).collect();
        println!("  Preview: {}", preview.replace('\n', " \\n "));
    }
}

#[cfg(test)]
mod scoring_tests {
    use super::*;

    #[test]
    fn test_word_f1_identical() {
        let (p, r, f1) = word_f1("hello world", "hello world");
        assert!((p - 1.0).abs() < 0.001);
        assert!((r - 1.0).abs() < 0.001);
        assert!((f1 - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_word_f1_no_overlap() {
        let (p, r, f1) = word_f1("hello world", "foo bar");
        assert!(p < 0.001);
        assert!(r < 0.001);
        assert!(f1 < 0.001);
    }

    #[test]
    fn test_word_f1_partial_overlap() {
        let (p, r, f1) = word_f1("hello world foo", "hello world bar");
        assert!(p > 0.5);
        assert!(r > 0.5);
        assert!(f1 > 0.5);
    }

    #[test]
    fn test_word_f1_empty() {
        let (_, _, f1) = word_f1("", "");
        assert!((f1 - 1.0).abs() < 0.001);

        let (_, _, f1) = word_f1("hello", "");
        assert!(f1 < 0.001);

        let (_, _, f1) = word_f1("", "hello");
        assert!(f1 < 0.001);
    }

    #[test]
    fn test_word_f1_case_insensitive() {
        let (_, _, f1) = word_f1("Hello World", "hello world");
        assert!((f1 - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_word_f1_punctuation_stripped() {
        let (_, _, f1) = word_f1("hello, world!", "hello world");
        assert!((f1 - 1.0).abs() < 0.001);
    }

    #[test]
    fn should_return_none_when_pdf_name_matches_no_candidate_path() {
        assert_eq!(
            resolve_pdf_path("nonexistent_document_12345"),
            None,
            "a name matching none of the candidate roots must resolve to None"
        );
    }
}
