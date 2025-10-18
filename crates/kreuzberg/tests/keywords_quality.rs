//! Keyword extraction quality assessment tests.
//!
//! This module tests keyword extraction quality by comparing against ground truth keywords.
//! Measures precision, recall, and F1 to ensure default configurations work well out of the box.
//!
//! Test philosophy:
//! - Define ground truth keywords for test documents (domain experts would identify these)
//! - Measure how well extracted keywords match ground truth
//! - Assert minimum quality thresholds for precision/recall/F1
//! - Verify domain relevance of extracted terms

#[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
use kreuzberg::keywords::{KeywordConfig, extract_keywords};
use std::collections::HashSet;

// ============================================================================
// Ground Truth Keywords (What domain experts would identify)
// ============================================================================

/// Ground truth keywords for ML document.
/// These are the terms a machine learning expert would identify as key concepts.
#[allow(dead_code)]
fn get_ml_ground_truth() -> HashSet<&'static str> {
    [
        // Core concepts
        "machine learning",
        "artificial intelligence",
        "deep learning",
        "neural networks",
        "artificial neural networks",
        "convolutional neural networks",
        // Key terms
        "algorithms",
        "training data",
        "supervised learning",
        "unsupervised learning",
        "semi-supervised",
        "natural language processing",
        "computer science",
        "model",
        // Related
        "predictions",
        "data",
        "learning",
    ]
    .iter()
    .cloned()
    .collect()
}

/// Ground truth keywords for climate change document.
#[allow(dead_code)]
fn get_climate_ground_truth() -> HashSet<&'static str> {
    [
        // Core concepts
        "climate change",
        "global warming",
        "greenhouse gases",
        "greenhouse gas emissions",
        "fossil fuels",
        "burning fossil fuels",
        // Key terms
        "carbon dioxide",
        "methane",
        "temperatures",
        "weather patterns",
        "climate system",
        "human activities",
        // Related
        "agriculture",
        "deforestation",
        "solar cycle",
        "earth",
    ]
    .iter()
    .cloned()
    .collect()
}

// ============================================================================
// Quality Metrics Functions
// ============================================================================

#[derive(Debug)]
#[allow(dead_code)]
struct KeywordQualityScores {
    precision: f64,
    recall: f64,
    f1: f64,
    exact_matches: usize,
    partial_matches: usize,
    total_extracted: usize,
    total_ground_truth: usize,
}

impl KeywordQualityScores {
    fn new(exact_matches: usize, partial_matches: usize, total_extracted: usize, total_ground_truth: usize) -> Self {
        // Precision: Of extracted keywords, how many are relevant?
        let precision = if total_extracted > 0 {
            (exact_matches + partial_matches) as f64 / total_extracted as f64
        } else {
            0.0
        };

        // Recall: Of ground truth keywords, how many were extracted?
        let recall = if total_ground_truth > 0 {
            (exact_matches + partial_matches) as f64 / total_ground_truth as f64
        } else {
            0.0
        };

        // F1: Harmonic mean of precision and recall
        let f1 = if precision + recall > 0.0 {
            2.0 * precision * recall / (precision + recall)
        } else {
            0.0
        };

        Self {
            precision,
            recall,
            f1,
            exact_matches,
            partial_matches,
            total_extracted,
            total_ground_truth,
        }
    }
}

/// Evaluate extracted keywords against ground truth.
///
/// Supports both exact matches and partial matches:
/// - Exact: "machine learning" == "machine learning"
/// - Partial: "machine" matches "machine learning" (subset)
#[allow(dead_code)]
fn evaluate_keyword_quality(extracted: &[&str], ground_truth: &HashSet<&str>) -> KeywordQualityScores {
    let extracted_lower: Vec<String> = extracted.iter().map(|s| s.to_lowercase()).collect();
    let ground_truth_lower: HashSet<String> = ground_truth.iter().map(|s| s.to_lowercase()).collect();

    let mut exact_matches = 0;
    let mut partial_matches = 0;
    let mut matched_ground_truth: HashSet<String> = HashSet::new();

    for extracted_kw in &extracted_lower {
        // Check for exact match
        if ground_truth_lower.contains(extracted_kw) {
            exact_matches += 1;
            matched_ground_truth.insert(extracted_kw.clone());
            continue;
        }

        // Check for partial match (extracted keyword is substring of ground truth)
        let mut found_partial = false;
        for gt_kw in &ground_truth_lower {
            if (gt_kw.contains(extracted_kw) || extracted_kw.contains(gt_kw)) && !matched_ground_truth.contains(gt_kw) {
                partial_matches += 1;
                matched_ground_truth.insert(gt_kw.clone());
                found_partial = true;
                break;
            }
        }

        if !found_partial {
            // Also check if extracted keyword contains important ground truth terms
            for gt_kw in &ground_truth_lower {
                let gt_words: Vec<&str> = gt_kw.split_whitespace().collect();
                let ex_words: HashSet<&str> = extracted_kw.split_whitespace().collect();

                // If extracted keyword contains significant overlap with ground truth
                let overlap = gt_words.iter().filter(|w| ex_words.contains(*w)).count();
                if overlap >= gt_words.len() / 2 && overlap > 0 && !matched_ground_truth.contains(gt_kw) {
                    partial_matches += 1;
                    matched_ground_truth.insert(gt_kw.clone());
                    break;
                }
            }
        }
    }

    KeywordQualityScores::new(
        exact_matches,
        partial_matches,
        extracted_lower.len(),
        ground_truth_lower.len(),
    )
}

/// ML document text (subset for testing).
#[allow(dead_code)]
const ML_DOC_SAMPLE: &str = r#"
Machine learning is a branch of artificial intelligence and computer science which focuses on the use of data and algorithms to imitate the way that humans learn.
Machine learning algorithms build a model based on sample data, known as training data, to make predictions or decisions without being explicitly programmed to do so.
Deep learning is a type of machine learning based on artificial neural networks. The learning process is deep because the structure of artificial neural networks consists of multiple input, output, and hidden layers.
Neural networks can be used for supervised, semi-supervised, and unsupervised learning. Convolutional neural networks are commonly applied to analyzing visual imagery.
Natural language processing is a subfield of linguistics, computer science, and artificial intelligence concerned with the interactions between computers and human language.
"#;

/// Climate document text (subset for testing).
#[allow(dead_code)]
const CLIMATE_DOC_SAMPLE: &str = r#"
Climate change refers to long-term shifts in temperatures and weather patterns. These shifts may be natural, such as through variations in the solar cycle.
But since the 1800s, human activities have been the main driver of climate change, primarily due to burning fossil fuels like coal, oil, and gas.
Burning fossil fuels generates greenhouse gas emissions that act like a blanket wrapped around the Earth, trapping the sun's heat and raising temperatures.
The main greenhouse gases that are causing climate change include carbon dioxide and methane. These come from burning fossil fuels for energy, agriculture, and deforestation.
Global warming is the long-term heating of Earth's climate system. Climate science reveals that human activity has been the dominant cause of climate change since the mid-20th century.
"#;

// ============================================================================
// Quality Tests with Default Configurations
// ============================================================================

#[cfg(feature = "keywords-yake")]
#[test]
fn test_yake_quality_ml_document_default_config() {
    let config = KeywordConfig::yake(); // Default YAKE config
    let keywords = extract_keywords(ML_DOC_SAMPLE, &config).unwrap();

    assert!(!keywords.is_empty(), "Should extract keywords with default config");

    // Evaluate against ground truth
    let extracted: Vec<&str> = keywords.iter().map(|k| k.text.as_str()).collect();
    let ground_truth = get_ml_ground_truth();
    let scores = evaluate_keyword_quality(&extracted, &ground_truth);

    println!("\nYAKE ML Document Quality (Default Config):");
    println!("  Extracted: {} keywords", scores.total_extracted);
    println!("  Ground truth: {} keywords", scores.total_ground_truth);
    println!("  Exact matches: {}", scores.exact_matches);
    println!("  Partial matches: {}", scores.partial_matches);
    println!("  Precision: {:.3}", scores.precision);
    println!("  Recall: {:.3}", scores.recall);
    println!("  F1: {:.3}", scores.f1);
    println!("\nExtracted keywords:");
    for (i, kw) in keywords.iter().enumerate().take(10) {
        println!("  {}: {} (score: {:.3})", i + 1, kw.text, kw.score);
    }

    // Assert minimum quality thresholds for default config
    assert!(
        scores.precision >= 0.40,
        "YAKE precision too low with default config: {:.3} (expected >= 0.40). Only {}/{} keywords were relevant.",
        scores.precision,
        scores.exact_matches + scores.partial_matches,
        scores.total_extracted
    );

    assert!(
        scores.recall >= 0.30,
        "YAKE recall too low with default config: {:.3} (expected >= 0.30). Only {}/{} ground truth keywords found.",
        scores.recall,
        scores.exact_matches + scores.partial_matches,
        scores.total_ground_truth
    );

    assert!(
        scores.f1 >= 0.30,
        "YAKE F1 score too low with default config: {:.3} (expected >= 0.30). Precision: {:.3}, Recall: {:.3}",
        scores.f1,
        scores.precision,
        scores.recall
    );
}

#[cfg(feature = "keywords-rake")]
#[test]
fn test_rake_quality_ml_document_default_config() {
    let config = KeywordConfig::rake(); // Default RAKE config
    let keywords = extract_keywords(ML_DOC_SAMPLE, &config).unwrap();

    assert!(!keywords.is_empty(), "Should extract keywords with default config");

    // Evaluate against ground truth
    let extracted: Vec<&str> = keywords.iter().map(|k| k.text.as_str()).collect();
    let ground_truth = get_ml_ground_truth();
    let scores = evaluate_keyword_quality(&extracted, &ground_truth);

    println!("\nRAKE ML Document Quality (Default Config):");
    println!("  Extracted: {} keywords", scores.total_extracted);
    println!("  Ground truth: {} keywords", scores.total_ground_truth);
    println!("  Exact matches: {}", scores.exact_matches);
    println!("  Partial matches: {}", scores.partial_matches);
    println!("  Precision: {:.3}", scores.precision);
    println!("  Recall: {:.3}", scores.recall);
    println!("  F1: {:.3}", scores.f1);
    println!("\nExtracted keywords:");
    for (i, kw) in keywords.iter().enumerate().take(10) {
        println!("  {}: {} (score: {:.3})", i + 1, kw.text, kw.score);
    }

    // Assert minimum quality thresholds for default config
    assert!(
        scores.precision >= 0.40,
        "RAKE precision too low with default config: {:.3} (expected >= 0.40). Only {}/{} keywords were relevant.",
        scores.precision,
        scores.exact_matches + scores.partial_matches,
        scores.total_extracted
    );

    assert!(
        scores.recall >= 0.30,
        "RAKE recall too low with default config: {:.3} (expected >= 0.30). Only {}/{} ground truth keywords found.",
        scores.recall,
        scores.exact_matches + scores.partial_matches,
        scores.total_ground_truth
    );

    assert!(
        scores.f1 >= 0.30,
        "RAKE F1 score too low with default config: {:.3} (expected >= 0.30). Precision: {:.3}, Recall: {:.3}",
        scores.f1,
        scores.precision,
        scores.recall
    );
}

#[cfg(feature = "keywords-yake")]
#[test]
fn test_yake_quality_climate_document_default_config() {
    let config = KeywordConfig::yake();
    let keywords = extract_keywords(CLIMATE_DOC_SAMPLE, &config).unwrap();

    assert!(!keywords.is_empty(), "Should extract keywords with default config");

    let extracted: Vec<&str> = keywords.iter().map(|k| k.text.as_str()).collect();
    let ground_truth = get_climate_ground_truth();
    let scores = evaluate_keyword_quality(&extracted, &ground_truth);

    println!("\nYAKE Climate Document Quality (Default Config):");
    println!("  Extracted: {} keywords", scores.total_extracted);
    println!("  Ground truth: {} keywords", scores.total_ground_truth);
    println!("  Exact matches: {}", scores.exact_matches);
    println!("  Partial matches: {}", scores.partial_matches);
    println!("  Precision: {:.3}", scores.precision);
    println!("  Recall: {:.3}", scores.recall);
    println!("  F1: {:.3}", scores.f1);
    println!("\nExtracted keywords:");
    for (i, kw) in keywords.iter().enumerate().take(10) {
        println!("  {}: {} (score: {:.3})", i + 1, kw.text, kw.score);
    }

    // Climate keywords should be well extracted
    assert!(
        scores.precision >= 0.40,
        "YAKE precision too low: {:.3} (expected >= 0.40)",
        scores.precision
    );
    assert!(
        scores.recall >= 0.30,
        "YAKE recall too low: {:.3} (expected >= 0.30)",
        scores.recall
    );
    assert!(
        scores.f1 >= 0.30,
        "YAKE F1 too low: {:.3} (expected >= 0.30)",
        scores.f1
    );
}

#[cfg(feature = "keywords-rake")]
#[test]
fn test_rake_quality_climate_document_default_config() {
    let config = KeywordConfig::rake();
    let keywords = extract_keywords(CLIMATE_DOC_SAMPLE, &config).unwrap();

    assert!(!keywords.is_empty(), "Should extract keywords with default config");

    let extracted: Vec<&str> = keywords.iter().map(|k| k.text.as_str()).collect();
    let ground_truth = get_climate_ground_truth();
    let scores = evaluate_keyword_quality(&extracted, &ground_truth);

    println!("\nRAKE Climate Document Quality (Default Config):");
    println!("  Extracted: {} keywords", scores.total_extracted);
    println!("  Ground truth: {} keywords", scores.total_ground_truth);
    println!("  Exact matches: {}", scores.exact_matches);
    println!("  Partial matches: {}", scores.partial_matches);
    println!("  Precision: {:.3}", scores.precision);
    println!("  Recall: {:.3}", scores.recall);
    println!("  F1: {:.3}", scores.f1);
    println!("\nExtracted keywords:");
    for (i, kw) in keywords.iter().enumerate().take(10) {
        println!("  {}: {} (score: {:.3})", i + 1, kw.text, kw.score);
    }

    // Climate keywords should be well extracted
    assert!(
        scores.precision >= 0.40,
        "RAKE precision too low: {:.3} (expected >= 0.40)",
        scores.precision
    );
    assert!(
        scores.recall >= 0.30,
        "RAKE recall too low: {:.3} (expected >= 0.30)",
        scores.recall
    );
    assert!(
        scores.f1 >= 0.30,
        "RAKE F1 too low: {:.3} (expected >= 0.30)",
        scores.f1
    );
}

// ============================================================================
// Comparative Quality Tests
// ============================================================================

#[cfg(all(feature = "keywords-yake", feature = "keywords-rake"))]
#[test]
fn test_yake_vs_rake_quality_comparison() {
    let yake_config = KeywordConfig::yake();
    let rake_config = KeywordConfig::rake();

    // Test on ML document
    let yake_keywords = extract_keywords(ML_DOC_SAMPLE, &yake_config).unwrap();
    let rake_keywords = extract_keywords(ML_DOC_SAMPLE, &rake_config).unwrap();

    let yake_extracted: Vec<&str> = yake_keywords.iter().map(|k| k.text.as_str()).collect();
    let rake_extracted: Vec<&str> = rake_keywords.iter().map(|k| k.text.as_str()).collect();

    let ground_truth = get_ml_ground_truth();
    let yake_scores = evaluate_keyword_quality(&yake_extracted, &ground_truth);
    let rake_scores = evaluate_keyword_quality(&rake_extracted, &ground_truth);

    println!("\nYAKE vs RAKE Quality Comparison (ML Document):");
    println!(
        "  YAKE F1: {:.3} (P: {:.3}, R: {:.3})",
        yake_scores.f1, yake_scores.precision, yake_scores.recall
    );
    println!(
        "  RAKE F1: {:.3} (P: {:.3}, R: {:.3})",
        rake_scores.f1, rake_scores.precision, rake_scores.recall
    );

    // Both algorithms should produce reasonable quality
    assert!(yake_scores.f1 >= 0.25, "YAKE F1 too low: {:.3}", yake_scores.f1);
    assert!(rake_scores.f1 >= 0.25, "RAKE F1 too low: {:.3}", rake_scores.f1);

    // At least one should achieve good quality
    let best_f1 = yake_scores.f1.max(rake_scores.f1);
    assert!(
        best_f1 >= 0.30,
        "Neither algorithm achieved F1 >= 0.30. Best: {:.3}",
        best_f1
    );
}

// ============================================================================
// Optimized Configuration Quality Tests
// ============================================================================

#[cfg(feature = "keywords-yake")]
#[test]
fn test_yake_quality_with_optimized_config() {
    // Optimized config: more keywords, broader ngram range
    let config = KeywordConfig::yake()
        .with_max_keywords(15)
        .with_ngram_range(1, 3)
        .with_min_score(0.0);

    let keywords = extract_keywords(ML_DOC_SAMPLE, &config).unwrap();

    let extracted: Vec<&str> = keywords.iter().map(|k| k.text.as_str()).collect();
    let ground_truth = get_ml_ground_truth();
    let scores = evaluate_keyword_quality(&extracted, &ground_truth);

    println!("\nYAKE ML Document Quality (Optimized Config - max 15, ngrams 1-3):");
    println!(
        "  F1: {:.3} (P: {:.3}, R: {:.3})",
        scores.f1, scores.precision, scores.recall
    );

    // Optimized config should improve recall
    assert!(
        scores.recall >= 0.35,
        "Optimized config should improve recall: {:.3} (expected >= 0.35)",
        scores.recall
    );
}

#[cfg(feature = "keywords-rake")]
#[test]
fn test_rake_quality_with_optimized_config() {
    // Optimized config: more keywords, broader ngram range
    let config = KeywordConfig::rake()
        .with_max_keywords(15)
        .with_ngram_range(1, 3)
        .with_min_score(0.0);

    let keywords = extract_keywords(ML_DOC_SAMPLE, &config).unwrap();

    let extracted: Vec<&str> = keywords.iter().map(|k| k.text.as_str()).collect();
    let ground_truth = get_ml_ground_truth();
    let scores = evaluate_keyword_quality(&extracted, &ground_truth);

    println!("\nRAKE ML Document Quality (Optimized Config - max 15, ngrams 1-3):");
    println!(
        "  F1: {:.3} (P: {:.3}, R: {:.3})",
        scores.f1, scores.precision, scores.recall
    );

    // Optimized config should improve recall
    assert!(
        scores.recall >= 0.35,
        "Optimized config should improve recall: {:.3} (expected >= 0.35)",
        scores.recall
    );
}

// ============================================================================
// Domain Relevance Tests
// ============================================================================

#[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
#[test]
fn test_extracted_keywords_are_domain_relevant() {
    let config = KeywordConfig::default();
    let keywords = extract_keywords(ML_DOC_SAMPLE, &config).unwrap();

    // Define domain-relevant terms
    let ml_terms = [
        "machine",
        "learning",
        "artificial",
        "intelligence",
        "neural",
        "network",
        "deep",
        "algorithm",
        "data",
        "model",
        "training",
        "supervised",
        "unsupervised",
        "language",
        "processing",
    ];

    // Check that extracted keywords contain domain-relevant terms
    let relevant_count = keywords
        .iter()
        .filter(|kw| {
            let kw_lower = kw.text.to_lowercase();
            ml_terms.iter().any(|term| kw_lower.contains(term))
        })
        .count();

    let relevance_ratio = relevant_count as f64 / keywords.len() as f64;

    println!("\nDomain Relevance Check:");
    println!("  Extracted keywords: {}", keywords.len());
    println!("  Domain-relevant keywords: {}", relevant_count);
    println!("  Relevance ratio: {:.3}", relevance_ratio);

    // At least 70% of extracted keywords should contain domain-relevant terms
    assert!(
        relevance_ratio >= 0.70,
        "Too many irrelevant keywords extracted. Relevance: {:.3} (expected >= 0.70). Relevant: {}/{}",
        relevance_ratio,
        relevant_count,
        keywords.len()
    );
}
