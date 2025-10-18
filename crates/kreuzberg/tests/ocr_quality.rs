//! OCR quality assessment tests.
//!
//! This module tests OCR quality by comparing against ground truth (native PDF text layers).
//! Adopts techniques from scripts/ocr_quality_report.py:
//! - Token-based precision/recall/F1 scoring
//! - Numeric accuracy tracking (critical for tables, data)
//! - Layout fidelity (line count preservation)
//! - Markdown structure preservation
//!
//! Test philosophy:
//! - Compare OCR output against searchable PDF text (ground truth)
//! - Measure accuracy with precision, recall, F1 metrics
//! - Track numeric token accuracy separately (higher importance)
//! - Verify layout preservation (line counts, structure)
//! - Assert minimum quality thresholds

mod helpers;

use helpers::*;
use kreuzberg::core::config::{ExtractionConfig, OcrConfig};
use kreuzberg::extract_file_sync;
use std::collections::HashMap;

// ============================================================================
// Quality Metrics Structures
// ============================================================================

#[derive(Debug, Clone)]
struct TokenScores {
    precision: f64,
    recall: f64,
    f1: f64,
}

impl TokenScores {
    fn new(precision: f64, recall: f64) -> Self {
        let f1 = if precision + recall == 0.0 {
            0.0
        } else {
            2.0 * precision * recall / (precision + recall)
        };
        Self { precision, recall, f1 }
    }
}

// ============================================================================
// Token Processing Functions
// ============================================================================

/// Tokenize and normalize text for comparison.
/// Matches Python implementation: lowercase, normalize dashes, remove punctuation.
fn tokenize_text(text: &str) -> HashMap<String, usize> {
    let normalized = text
        .to_lowercase()
        .replace(['\u{2013}', '\u{2014}'], "-") // em dash
        .chars()
        .map(|ch| {
            if ch >= ' ' || ch == '\n' || ch == '\r' || ch == '\t' {
                ch
            } else {
                ' '
            }
        })
        .collect::<String>();

    // Remove punctuation
    let normalized = normalized
        .chars()
        .map(|ch| if "()[],.;:+`".contains(ch) { ' ' } else { ch })
        .collect::<String>();

    let mut tokens: HashMap<String, usize> = HashMap::new();
    for token in normalized.split_whitespace() {
        *tokens.entry(token.to_string()).or_insert(0) += 1;
    }
    tokens
}

/// Extract numeric tokens from token map.
/// Critical for measuring accuracy on tables, data, figures.
fn extract_numeric_tokens(tokens: &HashMap<String, usize>) -> HashMap<String, usize> {
    let mut numeric_tokens: HashMap<String, usize> = HashMap::new();

    for (token, count) in tokens {
        let stripped = token.trim_matches(|c: char| "()[]{}".contains(c));

        // Must contain digit
        if !stripped.chars().any(|ch| ch.is_ascii_digit()) {
            continue;
        }

        // Must not contain letters (pure numeric)
        if stripped.chars().any(|ch| ch.is_ascii_alphabetic()) {
            continue;
        }

        *numeric_tokens.entry(stripped.to_string()).or_insert(0) += count;
    }

    numeric_tokens
}

/// Calculate precision, recall, F1 for token sets.
fn calculate_token_scores(
    truth_tokens: &HashMap<String, usize>,
    candidate_tokens: &HashMap<String, usize>,
) -> TokenScores {
    let truth_total: usize = truth_tokens.values().sum();
    let candidate_total: usize = candidate_tokens.values().sum();

    if truth_total == 0 && candidate_total == 0 {
        return TokenScores::new(1.0, 1.0);
    }

    // Calculate overlap (minimum count for each token)
    let overlap: usize = truth_tokens
        .keys()
        .map(|token| {
            let truth_count = truth_tokens.get(token).unwrap_or(&0);
            let candidate_count = candidate_tokens.get(token).unwrap_or(&0);
            truth_count.min(candidate_count)
        })
        .sum();

    let precision = if candidate_total > 0 {
        overlap as f64 / candidate_total as f64
    } else {
        0.0
    };

    let recall = if truth_total > 0 {
        overlap as f64 / truth_total as f64
    } else {
        0.0
    };

    TokenScores::new(precision, recall)
}

/// Count non-empty lines in text (layout fidelity metric).
fn count_lines(text: &str) -> usize {
    text.lines().filter(|line| !line.trim().is_empty()).count()
}

/// Calculate relative layout delta (0.0 = perfect, 1.0 = worst).
fn layout_delta(truth_lines: usize, ocr_lines: usize) -> f64 {
    if truth_lines == 0 {
        return if ocr_lines == 0 { 0.0 } else { 1.0 };
    }

    let delta = (ocr_lines as f64 - truth_lines as f64).abs() / truth_lines as f64;
    delta.min(1.0)
}

// ============================================================================
// Quality Threshold Tests
// ============================================================================

#[test]
fn test_ocr_quality_simple_text_high_accuracy() {
    if skip_if_missing("pdfs/fake_memo.pdf") {
        return;
    }

    let file_path = get_test_file_path("pdfs/fake_memo.pdf");

    // Extract native text (ground truth)
    let truth_result =
        extract_file_sync(&file_path, None, &ExtractionConfig::default()).expect("Should extract ground truth text");

    // Verify ExtractionResult structure for ground truth
    assert!(
        truth_result.chunks.is_none(),
        "Chunks should be None without chunking config"
    );
    assert!(
        truth_result.detected_languages.is_none(),
        "Language detection not enabled"
    );

    // Extract with forced OCR
    let ocr_config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: "eng".to_string(),
            tesseract_config: None,
        }),
        force_ocr: true,
        ..Default::default()
    };

    let ocr_result = extract_file_sync(&file_path, None, &ocr_config).expect("Should extract with OCR");

    // Verify ExtractionResult structure for OCR result
    assert!(
        ocr_result.chunks.is_none(),
        "Chunks should be None without chunking config"
    );
    assert!(
        ocr_result.detected_languages.is_none(),
        "Language detection not enabled"
    );

    // Debug: Check if force_ocr actually ran
    println!("Truth content length: {}", truth_result.content.len());
    println!("OCR content length: {}", ocr_result.content.len());
    println!(
        "Truth first 100 chars: {:?}",
        &truth_result.content.chars().take(100).collect::<String>()
    );
    println!(
        "OCR first 100 chars: {:?}",
        &ocr_result.content.chars().take(100).collect::<String>()
    );

    // Tokenize and score
    let truth_tokens = tokenize_text(&truth_result.content);
    let ocr_tokens = tokenize_text(&ocr_result.content);

    println!("Truth token count: {}", truth_tokens.len());
    println!("OCR token count: {}", ocr_tokens.len());

    let scores = calculate_token_scores(&truth_tokens, &ocr_tokens);

    println!("Simple text OCR quality:");
    println!("  Precision: {:.3}", scores.precision);
    println!("  Recall: {:.3}", scores.recall);
    println!("  F1: {:.3}", scores.f1);

    // Assert minimum quality thresholds
    assert!(
        scores.f1 >= 0.70,
        "OCR F1 score too low: {:.3} (expected >= 0.70). Precision: {:.3}, Recall: {:.3}",
        scores.f1,
        scores.precision,
        scores.recall
    );
}

#[test]
fn test_ocr_quality_numeric_accuracy() {
    if skip_if_missing("pdfs/embedded_images_tables.pdf") {
        return;
    }

    let file_path = get_test_file_path("pdfs/embedded_images_tables.pdf");

    // Extract native text (ground truth)
    let truth_result =
        extract_file_sync(&file_path, None, &ExtractionConfig::default()).expect("Should extract ground truth text");

    // Verify ExtractionResult structure for ground truth
    assert!(
        truth_result.chunks.is_none(),
        "Chunks should be None without chunking config"
    );
    assert!(
        truth_result.detected_languages.is_none(),
        "Language detection not enabled"
    );

    // Extract with forced OCR
    let ocr_config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: "eng".to_string(),
            tesseract_config: None,
        }),
        force_ocr: true,
        ..Default::default()
    };

    let ocr_result = extract_file_sync(&file_path, None, &ocr_config).expect("Should extract with OCR");

    // Verify ExtractionResult structure for OCR result
    assert!(
        ocr_result.chunks.is_none(),
        "Chunks should be None without chunking config"
    );
    assert!(
        ocr_result.detected_languages.is_none(),
        "Language detection not enabled"
    );

    // Extract numeric tokens only
    let truth_tokens = tokenize_text(&truth_result.content);
    let ocr_tokens = tokenize_text(&ocr_result.content);

    let truth_numeric = extract_numeric_tokens(&truth_tokens);
    let ocr_numeric = extract_numeric_tokens(&ocr_tokens);

    // Only test if document contains numbers
    if !truth_numeric.is_empty() {
        let numeric_scores = calculate_token_scores(&truth_numeric, &ocr_numeric);

        println!("Numeric token OCR quality:");
        println!("  Precision: {:.3}", numeric_scores.precision);
        println!("  Recall: {:.3}", numeric_scores.recall);
        println!("  F1: {:.3}", numeric_scores.f1);
        println!("  Numeric tokens in truth: {}", truth_numeric.len());
        println!("  Numeric tokens in OCR: {}", ocr_numeric.len());

        // Numeric accuracy is critical - higher threshold
        assert!(
            numeric_scores.f1 >= 0.75,
            "Numeric F1 score too low: {:.3} (expected >= 0.75). Numbers must be accurate!",
            numeric_scores.f1
        );
    }
}

#[test]
fn test_ocr_quality_layout_preservation() {
    if skip_if_missing("pdfs/fake_memo.pdf") {
        return;
    }

    let file_path = get_test_file_path("pdfs/fake_memo.pdf");

    // Extract native text (ground truth)
    let truth_result =
        extract_file_sync(&file_path, None, &ExtractionConfig::default()).expect("Should extract ground truth text");

    // Verify ExtractionResult structure for ground truth
    assert!(
        truth_result.chunks.is_none(),
        "Chunks should be None without chunking config"
    );
    assert!(
        truth_result.detected_languages.is_none(),
        "Language detection not enabled"
    );

    // Extract with forced OCR
    let ocr_config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: "eng".to_string(),
            tesseract_config: None,
        }),
        force_ocr: true,
        ..Default::default()
    };

    let ocr_result = extract_file_sync(&file_path, None, &ocr_config).expect("Should extract with OCR");

    // Verify ExtractionResult structure for OCR result
    assert!(
        ocr_result.chunks.is_none(),
        "Chunks should be None without chunking config"
    );
    assert!(
        ocr_result.detected_languages.is_none(),
        "Language detection not enabled"
    );

    // Count lines
    let truth_lines = count_lines(&truth_result.content);
    let ocr_lines = count_lines(&ocr_result.content);
    let delta = layout_delta(truth_lines, ocr_lines);

    println!("Layout preservation:");
    println!("  Truth lines: {}", truth_lines);
    println!("  OCR lines: {}", ocr_lines);
    println!("  Layout delta: {:.3}", delta);

    // Layout should be reasonably preserved
    assert!(
        delta <= 0.40,
        "Layout delta too high: {:.3} (expected <= 0.40). Truth: {} lines, OCR: {} lines",
        delta,
        truth_lines,
        ocr_lines
    );
}

#[test]
fn test_ocr_quality_technical_document() {
    if skip_if_missing("pdfs/code_and_formula.pdf") {
        return;
    }

    let file_path = get_test_file_path("pdfs/code_and_formula.pdf");

    // Extract native text (ground truth)
    let truth_result =
        extract_file_sync(&file_path, None, &ExtractionConfig::default()).expect("Should extract ground truth text");

    // Verify ExtractionResult structure for ground truth
    assert!(
        truth_result.chunks.is_none(),
        "Chunks should be None without chunking config"
    );
    assert!(
        truth_result.detected_languages.is_none(),
        "Language detection not enabled"
    );

    // Extract with forced OCR
    let ocr_config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: "eng".to_string(),
            tesseract_config: None,
        }),
        force_ocr: true,
        ..Default::default()
    };

    let ocr_result = extract_file_sync(&file_path, None, &ocr_config).expect("Should extract with OCR");

    // Verify ExtractionResult structure for OCR result
    assert!(
        ocr_result.chunks.is_none(),
        "Chunks should be None without chunking config"
    );
    assert!(
        ocr_result.detected_languages.is_none(),
        "Language detection not enabled"
    );

    // Overall quality
    let truth_tokens = tokenize_text(&truth_result.content);
    let ocr_tokens = tokenize_text(&ocr_result.content);
    let scores = calculate_token_scores(&truth_tokens, &ocr_tokens);

    println!("Technical document OCR quality:");
    println!("  Precision: {:.3}", scores.precision);
    println!("  Recall: {:.3}", scores.recall);
    println!("  F1: {:.3}", scores.f1);

    // Technical docs (code, formulas) are harder - lower threshold
    assert!(
        scores.f1 >= 0.60,
        "Technical document F1 score too low: {:.3} (expected >= 0.60)",
        scores.f1
    );
}

// ============================================================================
// Consistency Tests (Multiple Runs)
// ============================================================================

#[test]
fn test_ocr_consistency_across_runs() {
    if skip_if_missing("pdfs/fake_memo.pdf") {
        return;
    }

    let file_path = get_test_file_path("pdfs/fake_memo.pdf");
    let ocr_config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: "eng".to_string(),
            tesseract_config: None,
        }),
        force_ocr: true,
        use_cache: false, // Disable cache to ensure fresh extraction
        ..Default::default()
    };

    // Run OCR 3 times
    let result1 = extract_file_sync(&file_path, None, &ocr_config).expect("First OCR run should succeed");
    let result2 = extract_file_sync(&file_path, None, &ocr_config).expect("Second OCR run should succeed");
    let result3 = extract_file_sync(&file_path, None, &ocr_config).expect("Third OCR run should succeed");

    // Verify ExtractionResult structure for all runs
    assert!(
        result1.chunks.is_none(),
        "Chunks should be None without chunking config"
    );
    assert!(result1.detected_languages.is_none(), "Language detection not enabled");
    assert!(
        result2.chunks.is_none(),
        "Chunks should be None without chunking config"
    );
    assert!(result2.detected_languages.is_none(), "Language detection not enabled");
    assert!(
        result3.chunks.is_none(),
        "Chunks should be None without chunking config"
    );
    assert!(result3.detected_languages.is_none(), "Language detection not enabled");

    // Tokenize all runs
    let tokens1 = tokenize_text(&result1.content);
    let tokens2 = tokenize_text(&result2.content);
    let tokens3 = tokenize_text(&result3.content);

    // Compare run1 vs run2
    let scores_1_2 = calculate_token_scores(&tokens1, &tokens2);
    let scores_1_3 = calculate_token_scores(&tokens1, &tokens3);

    println!("OCR consistency across runs:");
    println!("  Run1 vs Run2 F1: {:.3}", scores_1_2.f1);
    println!("  Run1 vs Run3 F1: {:.3}", scores_1_3.f1);

    // OCR should be deterministic - expect very high consistency
    assert!(
        scores_1_2.f1 >= 0.98,
        "OCR inconsistent between runs: F1 {:.3} (expected >= 0.98)",
        scores_1_2.f1
    );
    assert!(
        scores_1_3.f1 >= 0.98,
        "OCR inconsistent between runs: F1 {:.3} (expected >= 0.98)",
        scores_1_3.f1
    );
}

#[test]
fn test_ocr_consistency_with_different_psm() {
    if skip_if_missing("pdfs/fake_memo.pdf") {
        return;
    }

    let file_path = get_test_file_path("pdfs/fake_memo.pdf");

    // PSM 3 (auto)
    let config_psm3 = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: "eng".to_string(),
            tesseract_config: Some(kreuzberg::types::TesseractConfig {
                psm: 3,
                ..Default::default()
            }),
        }),
        force_ocr: true,
        ..Default::default()
    };

    // PSM 6 (single block)
    let config_psm6 = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: "eng".to_string(),
            tesseract_config: Some(kreuzberg::types::TesseractConfig {
                psm: 6,
                ..Default::default()
            }),
        }),
        force_ocr: true,
        ..Default::default()
    };

    let result_psm3 = extract_file_sync(&file_path, None, &config_psm3).expect("PSM 3 extraction should succeed");
    let result_psm6 = extract_file_sync(&file_path, None, &config_psm6).expect("PSM 6 extraction should succeed");

    // Verify ExtractionResult structure for both PSM modes
    assert!(
        result_psm3.chunks.is_none(),
        "Chunks should be None without chunking config"
    );
    assert!(
        result_psm3.detected_languages.is_none(),
        "Language detection not enabled"
    );
    assert!(
        result_psm6.chunks.is_none(),
        "Chunks should be None without chunking config"
    );
    assert!(
        result_psm6.detected_languages.is_none(),
        "Language detection not enabled"
    );

    let tokens_psm3 = tokenize_text(&result_psm3.content);
    let tokens_psm6 = tokenize_text(&result_psm6.content);

    let scores = calculate_token_scores(&tokens_psm3, &tokens_psm6);

    println!("OCR consistency across PSM modes:");
    println!("  PSM 3 vs PSM 6 F1: {:.3}", scores.f1);

    // Different PSM modes should still produce similar results
    assert!(
        scores.f1 >= 0.85,
        "PSM modes produce too different results: F1 {:.3} (expected >= 0.85)",
        scores.f1
    );
}

// ============================================================================
// Complex Document Quality Tests
// ============================================================================

#[test]
fn test_ocr_quality_multi_page_consistency() {
    if skip_if_missing("pdfs/a_course_in_machine_learning_ciml_v0_9_all.pdf") {
        return;
    }

    let file_path = get_test_file_path("pdfs/a_course_in_machine_learning_ciml_v0_9_all.pdf");

    // Extract native text (ground truth) - limited pages for performance
    let truth_result =
        extract_file_sync(&file_path, None, &ExtractionConfig::default()).expect("Should extract ground truth text");

    // Verify ExtractionResult structure for ground truth
    assert!(
        truth_result.chunks.is_none(),
        "Chunks should be None without chunking config"
    );
    assert!(
        truth_result.detected_languages.is_none(),
        "Language detection not enabled"
    );

    // Extract with forced OCR
    let ocr_config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: "eng".to_string(),
            tesseract_config: None,
        }),
        force_ocr: true,
        ..Default::default()
    };

    let ocr_result = extract_file_sync(&file_path, None, &ocr_config).expect("Should extract with OCR");

    // Verify ExtractionResult structure for OCR result
    assert!(
        ocr_result.chunks.is_none(),
        "Chunks should be None without chunking config"
    );
    assert!(
        ocr_result.detected_languages.is_none(),
        "Language detection not enabled"
    );

    // For large documents, just verify OCR produces reasonable output
    let truth_tokens = tokenize_text(&truth_result.content);
    let ocr_tokens = tokenize_text(&ocr_result.content);

    let truth_count: usize = truth_tokens.values().sum();
    let ocr_count: usize = ocr_tokens.values().sum();

    println!("Multi-page document quality:");
    println!("  Truth token count: {}", truth_count);
    println!("  OCR token count: {}", ocr_count);

    // OCR should extract substantial content from large docs
    assert!(
        ocr_count >= (truth_count * 50 / 100),
        "OCR extracted too few tokens: {} (expected >= 50% of {})",
        ocr_count,
        truth_count
    );
}

#[test]
fn test_ocr_quality_with_tables() {
    if skip_if_missing("pdfs/embedded_images_tables.pdf") {
        return;
    }

    let file_path = get_test_file_path("pdfs/embedded_images_tables.pdf");

    // Extract with table detection enabled
    let ocr_config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: "eng".to_string(),
            tesseract_config: Some(kreuzberg::types::TesseractConfig {
                enable_table_detection: true,
                table_min_confidence: 0.5,
                ..Default::default()
            }),
        }),
        force_ocr: true,
        ..Default::default()
    };

    let result = extract_file_sync(&file_path, None, &ocr_config).expect("Should extract with table detection");

    // Verify ExtractionResult structure
    assert!(result.chunks.is_none(), "Chunks should be None without chunking config");
    assert!(result.detected_languages.is_none(), "Language detection not enabled");

    println!("Table extraction quality:");
    println!("  Tables found: {}", result.tables.len());
    println!("  Content length: {}", result.content.len());

    // Verify OCR ran and produced content
    assert!(
        !result.content.trim().is_empty(),
        "OCR with tables should produce content"
    );
}
