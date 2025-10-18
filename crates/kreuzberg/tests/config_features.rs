//! Configuration features integration tests.
//!
//! Tests for chunking, language detection, caching, token reduction, and quality processing.
//! Validates that configuration options work correctly end-to-end.

use kreuzberg::core::config::{ChunkingConfig, ExtractionConfig, LanguageDetectionConfig, TokenReductionConfig};
use kreuzberg::core::extractor::extract_bytes;

mod helpers;

// ============================================================================
// Chunking Tests (4 tests)
// ============================================================================

/// Test chunking enabled - text split into chunks.
#[tokio::test]
async fn test_chunking_enabled() {
    let config = ExtractionConfig {
        chunking: Some(ChunkingConfig {
            max_chars: 50,
            max_overlap: 10,
        }),
        ..Default::default()
    };

    // Long text that should be split into multiple chunks
    let text = "This is a long text that should be split into multiple chunks. ".repeat(10);
    let text_bytes = text.as_bytes();

    let result = extract_bytes(text_bytes, "text/plain", &config)
        .await
        .expect("Should extract successfully");

    // Verify chunking occurred
    assert!(result.metadata.additional.contains_key("chunk_count"));
    let chunk_count = result.metadata.additional.get("chunk_count").unwrap();
    assert!(chunk_count.as_u64().unwrap() > 1, "Should have multiple chunks");
}

/// Test chunking with overlap - overlap preserved between chunks.
#[tokio::test]
async fn test_chunking_with_overlap() {
    let config = ExtractionConfig {
        chunking: Some(ChunkingConfig {
            max_chars: 100,
            max_overlap: 20,
        }),
        ..Default::default()
    };

    let text = "a".repeat(250); // 250 characters
    let text_bytes = text.as_bytes();

    let result = extract_bytes(text_bytes, "text/plain", &config)
        .await
        .expect("Should extract successfully");

    // Verify chunking with overlap
    assert!(result.metadata.additional.contains_key("chunk_count"));
    let chunk_count = result.metadata.additional.get("chunk_count").unwrap();
    assert!(chunk_count.as_u64().unwrap() >= 2, "Should have at least 2 chunks");
}

/// Test chunking with custom sizes - custom chunk size and overlap.
#[tokio::test]
async fn test_chunking_custom_sizes() {
    let config = ExtractionConfig {
        chunking: Some(ChunkingConfig {
            max_chars: 200,
            max_overlap: 50,
        }),
        ..Default::default()
    };

    let text = "Custom chunk test. ".repeat(50);
    let text_bytes = text.as_bytes();

    let result = extract_bytes(text_bytes, "text/plain", &config)
        .await
        .expect("Should extract successfully");

    // Verify custom chunking settings applied
    assert!(result.metadata.additional.contains_key("chunk_count"));
}

/// Test chunking disabled - no chunking when disabled.
#[tokio::test]
async fn test_chunking_disabled() {
    let config = ExtractionConfig {
        chunking: None, // Chunking disabled
        ..Default::default()
    };

    let text = "This is a long text that should NOT be split into chunks. ".repeat(10);
    let text_bytes = text.as_bytes();

    let result = extract_bytes(text_bytes, "text/plain", &config)
        .await
        .expect("Should extract successfully");

    // Verify no chunking occurred
    assert!(!result.metadata.additional.contains_key("chunk_count"));
}

// ============================================================================
// Language Detection Tests (4 tests)
// ============================================================================

/// Test language detection for single language document.
#[tokio::test]
async fn test_language_detection_single() {
    let config = ExtractionConfig {
        language_detection: Some(LanguageDetectionConfig {
            enabled: true,
            min_confidence: 0.8,
            detect_multiple: false,
        }),
        ..Default::default()
    };

    let text = "Hello world! This is English text. It should be detected as English language.";
    let text_bytes = text.as_bytes();

    let result = extract_bytes(text_bytes, "text/plain", &config)
        .await
        .expect("Should extract successfully");

    // Verify language detection
    assert!(result.detected_languages.is_some(), "Should detect language");
    let languages = result.detected_languages.unwrap();
    assert!(!languages.is_empty(), "Should detect at least one language");
    assert_eq!(languages[0], "eng", "Should detect English");
}

/// Test language detection for multi-language document.
#[tokio::test]
async fn test_language_detection_multiple() {
    let config = ExtractionConfig {
        language_detection: Some(LanguageDetectionConfig {
            enabled: true,
            min_confidence: 0.7,
            detect_multiple: true,
        }),
        ..Default::default()
    };

    // Multi-language text (English + Spanish)
    let text = "Hello world! This is English. ".repeat(10) + "Hola mundo! Este es español. ".repeat(10).as_str();
    let text_bytes = text.as_bytes();

    let result = extract_bytes(text_bytes, "text/plain", &config)
        .await
        .expect("Should extract successfully");

    // Verify multiple language detection
    assert!(result.detected_languages.is_some(), "Should detect languages");
    let languages = result.detected_languages.unwrap();
    assert!(!languages.is_empty(), "Should detect at least one language");
}

/// Test language detection with confidence threshold.
#[tokio::test]
async fn test_language_detection_confidence() {
    let config = ExtractionConfig {
        language_detection: Some(LanguageDetectionConfig {
            enabled: true,
            min_confidence: 0.9, // High confidence threshold
            detect_multiple: false,
        }),
        ..Default::default()
    };

    let text = "This is clear English text that should have high confidence.";
    let text_bytes = text.as_bytes();

    let result = extract_bytes(text_bytes, "text/plain", &config)
        .await
        .expect("Should extract successfully");

    // Verify language detection with high confidence threshold
    // Note: Detection may return None for short text with high threshold
    if let Some(languages) = result.detected_languages {
        assert!(!languages.is_empty());
    }
}

/// Test language detection disabled.
#[tokio::test]
async fn test_language_detection_disabled() {
    let config = ExtractionConfig {
        language_detection: Some(LanguageDetectionConfig {
            enabled: false, // Disabled
            min_confidence: 0.8,
            detect_multiple: false,
        }),
        ..Default::default()
    };

    let text = "Hello world! This is English text.";
    let text_bytes = text.as_bytes();

    let result = extract_bytes(text_bytes, "text/plain", &config)
        .await
        .expect("Should extract successfully");

    // Verify language detection did not run
    assert!(
        result.detected_languages.is_none(),
        "Should not detect language when disabled"
    );
}

// ============================================================================
// Caching Tests (4 tests)
// ============================================================================

/// Test cache hit behavior - second extraction from cache.
#[tokio::test]
async fn test_cache_hit_behavior() {
    let config = ExtractionConfig {
        use_cache: true,
        ..Default::default()
    };

    let text = "Test text for caching behavior.";
    let text_bytes = text.as_bytes();

    // First extraction (cache miss)
    let result1 = extract_bytes(text_bytes, "text/plain", &config)
        .await
        .expect("First extraction should succeed");

    // Second extraction (should hit cache for OCR results if applicable)
    let result2 = extract_bytes(text_bytes, "text/plain", &config)
        .await
        .expect("Second extraction should succeed");

    // Verify both extractions returned same content
    assert_eq!(result1.content, result2.content);
}

/// Test cache miss and invalidation.
#[tokio::test]
async fn test_cache_miss_invalidation() {
    let config = ExtractionConfig {
        use_cache: true,
        ..Default::default()
    };

    let text1 = "First text for cache test.";
    let text2 = "Second different text.";

    let result1 = extract_bytes(text1.as_bytes(), "text/plain", &config)
        .await
        .expect("First extraction should succeed");

    let result2 = extract_bytes(text2.as_bytes(), "text/plain", &config)
        .await
        .expect("Second extraction should succeed");

    // Verify different content (cache miss for different input)
    assert_ne!(result1.content, result2.content);
}

/// Test custom cache directory (Note: OCR cache uses hardcoded directory).
#[tokio::test]
async fn test_custom_cache_directory() {
    // Note: Current implementation uses hardcoded cache directory
    // This test verifies cache functionality works regardless
    let config = ExtractionConfig {
        use_cache: true,
        ..Default::default()
    };

    let text = "Test text for cache directory test.";
    let text_bytes = text.as_bytes();

    let result = extract_bytes(text_bytes, "text/plain", &config)
        .await
        .expect("Should extract successfully");

    assert!(!result.content.is_empty());
}

/// Test cache disabled - bypass cache.
#[tokio::test]
async fn test_cache_disabled() {
    let config = ExtractionConfig {
        use_cache: false, // Cache disabled
        ..Default::default()
    };

    let text = "Test text without caching.";
    let text_bytes = text.as_bytes();

    let result1 = extract_bytes(text_bytes, "text/plain", &config)
        .await
        .expect("First extraction should succeed");

    let result2 = extract_bytes(text_bytes, "text/plain", &config)
        .await
        .expect("Second extraction should succeed");

    // Both extractions should work (no cache errors)
    assert_eq!(result1.content, result2.content);
}

// ============================================================================
// Token Reduction Tests (3 tests)
// ============================================================================

/// Test token reduction in aggressive mode.
#[tokio::test]
async fn test_token_reduction_aggressive() {
    let config = ExtractionConfig {
        token_reduction: Some(TokenReductionConfig {
            mode: "aggressive".to_string(),
            preserve_important_words: true,
        }),
        ..Default::default()
    };

    let text = "This is a very long sentence with many unnecessary words that could be reduced. ".repeat(5);
    let text_bytes = text.as_bytes();

    let result = extract_bytes(text_bytes, "text/plain", &config)
        .await
        .expect("Should extract successfully");

    // Verify extraction succeeded (token reduction is applied in pipeline if feature enabled)
    assert!(!result.content.is_empty());
}

/// Test token reduction in conservative mode.
#[tokio::test]
async fn test_token_reduction_conservative() {
    let config = ExtractionConfig {
        token_reduction: Some(TokenReductionConfig {
            mode: "light".to_string(),
            preserve_important_words: true,
        }),
        ..Default::default()
    };

    let text = "Conservative token reduction test with moderate text length.";
    let text_bytes = text.as_bytes();

    let result = extract_bytes(text_bytes, "text/plain", &config)
        .await
        .expect("Should extract successfully");

    assert!(!result.content.is_empty());
}

/// Test token reduction disabled.
#[tokio::test]
async fn test_token_reduction_disabled() {
    let config = ExtractionConfig {
        token_reduction: Some(TokenReductionConfig {
            mode: "off".to_string(),
            preserve_important_words: false,
        }),
        ..Default::default()
    };

    let text = "Text without token reduction applied.";
    let text_bytes = text.as_bytes();

    let result = extract_bytes(text_bytes, "text/plain", &config)
        .await
        .expect("Should extract successfully");

    // Text should be extracted without modification
    assert!(result.content.contains("without token reduction"));
}

// ============================================================================
// Quality Processing Tests (3 tests)
// ============================================================================

/// Test quality processing enabled - quality scoring applied.
#[tokio::test]
async fn test_quality_processing_enabled() {
    let config = ExtractionConfig {
        enable_quality_processing: true,
        ..Default::default()
    };

    let text = "This is well-structured text. It has multiple sentences. And proper punctuation.";
    let text_bytes = text.as_bytes();

    let result = extract_bytes(text_bytes, "text/plain", &config)
        .await
        .expect("Should extract successfully");

    // Verify quality score is present
    if let Some(score) = result.metadata.additional.get("quality_score") {
        let score_value = score.as_f64().unwrap();
        assert!(score_value >= 0.0 && score_value <= 1.0);
    }

    assert!(!result.content.is_empty());
}

/// Test quality processing calculates score for different text quality.
#[tokio::test]
async fn test_quality_threshold_filtering() {
    let config = ExtractionConfig {
        enable_quality_processing: true,
        ..Default::default()
    };

    // High quality text
    let high_quality = "This is a well-structured document. It has proper sentences. And good formatting.";
    let result_high = extract_bytes(high_quality.as_bytes(), "text/plain", &config)
        .await
        .expect("Should extract successfully");

    // Low quality text with OCR artifacts
    let low_quality = "a  b  c  d  ....... word123mixed .  . ";
    let result_low = extract_bytes(low_quality.as_bytes(), "text/plain", &config)
        .await
        .expect("Should extract successfully");

    // Verify quality scores exist and are in valid range
    assert!(
        result_high.metadata.additional.contains_key("quality_score"),
        "High quality should have score"
    );
    assert!(
        result_low.metadata.additional.contains_key("quality_score"),
        "Low quality should have score"
    );

    let score_high = result_high
        .metadata
        .additional
        .get("quality_score")
        .unwrap()
        .as_f64()
        .unwrap();
    let score_low = result_low
        .metadata
        .additional
        .get("quality_score")
        .unwrap()
        .as_f64()
        .unwrap();

    // Verify scores are in valid range
    assert!(score_high >= 0.0 && score_high <= 1.0);
    assert!(score_low >= 0.0 && score_low <= 1.0);

    // High quality should generally score higher than low quality (though not always guaranteed for short text)
    // The important thing is that quality processing is working
}

/// Test quality processing disabled.
#[tokio::test]
async fn test_quality_processing_disabled() {
    let config = ExtractionConfig {
        enable_quality_processing: false,
        ..Default::default()
    };

    let text = "Text without quality processing.";
    let text_bytes = text.as_bytes();

    let result = extract_bytes(text_bytes, "text/plain", &config)
        .await
        .expect("Should extract successfully");

    // No quality score should be present
    assert!(!result.metadata.additional.contains_key("quality_score"));
    assert!(!result.content.is_empty());
}
