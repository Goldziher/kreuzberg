//! Configuration features integration tests.
//!
//! Tests for chunking, language detection, caching, token reduction, and quality processing.
//! Validates that configuration options work correctly end-to-end.

use kreuzberg::core::config::{ChunkingConfig, ExtractionConfig, LanguageDetectionConfig, TokenReductionConfig};
use kreuzberg::core::extractor::extract_bytes;

mod helpers;

/// Test chunking enabled - text split into chunks.
#[tokio::test]
async fn test_chunking_enabled() {
    let config = ExtractionConfig {
        chunking: Some(ChunkingConfig {
            max_chars: 50,
            max_overlap: 10,
            embedding: None,
            preset: None,
        }),
        ..Default::default()
    };

    let text = "This is a long text that should be split into multiple chunks. ".repeat(10);
    let text_bytes = text.as_bytes();

    let result = extract_bytes(text_bytes, "text/plain", &config)
        .await
        .expect("Should extract successfully");

    assert!(result.chunks.is_some(), "Chunks should be present");
    let chunks = result.chunks.unwrap();
    assert!(chunks.len() > 1, "Should have multiple chunks");

    assert!(result.metadata.additional.contains_key("chunk_count"));
    let chunk_count = result.metadata.additional.get("chunk_count").unwrap();
    assert_eq!(
        chunks.len(),
        chunk_count.as_u64().unwrap() as usize,
        "Chunks length should match chunk_count metadata"
    );

    for chunk in &chunks {
        assert!(!chunk.content.is_empty(), "Chunk should not be empty");
        assert!(
            chunk.content.len() <= 50 + 10,
            "Chunk length {} exceeds max_chars + overlap",
            chunk.content.len()
        );
    }
}

/// Test chunking with overlap - overlap preserved between chunks.
#[tokio::test]
async fn test_chunking_with_overlap() {
    let config = ExtractionConfig {
        chunking: Some(ChunkingConfig {
            max_chars: 100,
            max_overlap: 20,
            embedding: None,
            preset: None,
        }),
        ..Default::default()
    };

    let text = "a".repeat(250);
    let text_bytes = text.as_bytes();

    let result = extract_bytes(text_bytes, "text/plain", &config)
        .await
        .expect("Should extract successfully");

    assert!(result.chunks.is_some(), "Chunks should be present");
    let chunks = result.chunks.unwrap();
    assert!(chunks.len() >= 2, "Should have at least 2 chunks");

    assert!(result.metadata.additional.contains_key("chunk_count"));

    if chunks.len() >= 2 {
        let chunk1 = &chunks[0];
        let chunk2 = &chunks[1];

        let chunk1_end = &chunk1.content[chunk1.content.len().saturating_sub(20)..];
        assert!(
            chunk2.content.starts_with(chunk1_end)
                || chunk1_end.starts_with(&chunk2.content[..chunk1_end.len().min(chunk2.content.len())]),
            "Chunks should have overlap"
        );
    }
}

/// Test chunking with custom sizes - custom chunk size and overlap.
#[tokio::test]
async fn test_chunking_custom_sizes() {
    let config = ExtractionConfig {
        chunking: Some(ChunkingConfig {
            max_chars: 200,
            max_overlap: 50,
            embedding: None,
            preset: None,
        }),
        ..Default::default()
    };

    let text = "Custom chunk test. ".repeat(50);
    let text_bytes = text.as_bytes();

    let result = extract_bytes(text_bytes, "text/plain", &config)
        .await
        .expect("Should extract successfully");

    assert!(result.chunks.is_some(), "Chunks should be present");
    let chunks = result.chunks.unwrap();
    assert!(!chunks.is_empty(), "Should have at least 1 chunk");

    assert!(result.metadata.additional.contains_key("chunk_count"));

    for chunk in &chunks {
        assert!(
            chunk.content.len() <= 200 + 50,
            "Chunk length {} exceeds custom max_chars + overlap",
            chunk.content.len()
        );
    }
}

/// Test chunking disabled - no chunking when disabled.
#[tokio::test]
async fn test_chunking_disabled() {
    let config = ExtractionConfig {
        chunking: None,
        ..Default::default()
    };

    let text = "This is a long text that should NOT be split into chunks. ".repeat(10);
    let text_bytes = text.as_bytes();

    let result = extract_bytes(text_bytes, "text/plain", &config)
        .await
        .expect("Should extract successfully");

    assert!(result.chunks.is_none(), "Should not have chunks when chunking disabled");
    assert!(
        !result.metadata.additional.contains_key("chunk_count"),
        "Should not have chunk_count when chunking disabled"
    );

    assert!(!result.content.is_empty(), "Content should be extracted");
    assert!(result.content.contains("long text"), "Should contain original text");
}

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

    let text = "Hello world! This is English. ".repeat(10) + "Hola mundo! Este es español. ".repeat(10).as_str();
    let text_bytes = text.as_bytes();

    let result = extract_bytes(text_bytes, "text/plain", &config)
        .await
        .expect("Should extract successfully");

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
            min_confidence: 0.9,
            detect_multiple: false,
        }),
        ..Default::default()
    };

    let text = "This is clear English text that should have high confidence.";
    let text_bytes = text.as_bytes();

    let result = extract_bytes(text_bytes, "text/plain", &config)
        .await
        .expect("Should extract successfully");

    if let Some(languages) = result.detected_languages {
        assert!(!languages.is_empty());
    }
}

/// Test language detection disabled.
#[tokio::test]
async fn test_language_detection_disabled() {
    let config = ExtractionConfig {
        language_detection: Some(LanguageDetectionConfig {
            enabled: false,
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

    assert!(
        result.detected_languages.is_none(),
        "Should not detect language when disabled"
    );
}

/// Test cache hit behavior - second extraction from cache.
#[tokio::test]
async fn test_cache_hit_behavior() {
    let config = ExtractionConfig {
        use_cache: true,
        ..Default::default()
    };

    let text = "Test text for caching behavior.";
    let text_bytes = text.as_bytes();

    let result1 = extract_bytes(text_bytes, "text/plain", &config)
        .await
        .expect("First extraction should succeed");

    let result2 = extract_bytes(text_bytes, "text/plain", &config)
        .await
        .expect("Second extraction should succeed");

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

    assert_ne!(result1.content, result2.content);
}

/// Test custom cache directory (Note: OCR cache uses hardcoded directory).
#[tokio::test]
async fn test_custom_cache_directory() {
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
        use_cache: false,
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

    assert_eq!(result1.content, result2.content);
}

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

    assert!(result.content.contains("without token reduction"));
}

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

    if let Some(score) = result.metadata.additional.get("quality_score") {
        let score_value = score.as_f64().unwrap();
        assert!((0.0..=1.0).contains(&score_value));
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

    let high_quality = "This is a well-structured document. It has proper sentences. And good formatting.";
    let result_high = extract_bytes(high_quality.as_bytes(), "text/plain", &config)
        .await
        .expect("Should extract successfully");

    let low_quality = "a  b  c  d  ....... word123mixed .  . ";
    let result_low = extract_bytes(low_quality.as_bytes(), "text/plain", &config)
        .await
        .expect("Should extract successfully");

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

    assert!((0.0..=1.0).contains(&score_high));
    assert!((0.0..=1.0).contains(&score_low));
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

    assert!(!result.metadata.additional.contains_key("quality_score"));
    assert!(!result.content.is_empty());
}

/// Test chunking with embeddings using balanced preset.
#[tokio::test]
#[cfg(feature = "embeddings")]
async fn test_chunking_with_embeddings() {
    use kreuzberg::core::config::EmbeddingConfig;

    let config = ExtractionConfig {
        chunking: Some(ChunkingConfig {
            max_chars: 100,
            max_overlap: 20,
            embedding: Some(EmbeddingConfig::default()),
            preset: None,
        }),
        ..Default::default()
    };

    let text = "This is a test document for embedding generation. ".repeat(10);
    let text_bytes = text.as_bytes();

    let result = extract_bytes(text_bytes, "text/plain", &config)
        .await
        .expect("Should extract successfully");

    // Verify chunks are present
    assert!(result.chunks.is_some(), "Chunks should be present");
    let chunks = result.chunks.unwrap();
    assert!(chunks.len() > 1, "Should have multiple chunks");

    // Verify embeddings were generated
    println!("Metadata: {:?}", result.metadata.additional);

    if let Some(error) = result.metadata.additional.get("embedding_error") {
        panic!("Embedding generation failed: {}", error);
    }

    assert!(
        result.metadata.additional.contains_key("embeddings_generated"),
        "Should have embeddings_generated metadata"
    );
    assert_eq!(
        result.metadata.additional.get("embeddings_generated").unwrap(),
        &serde_json::Value::Bool(true)
    );

    // Verify each chunk has an embedding with correct dimensions
    for chunk in &chunks {
        assert!(chunk.embedding.is_some(), "Each chunk should have an embedding");
        let embedding = chunk.embedding.as_ref().unwrap();
        // Default config uses balanced preset (BGEBaseENV15) with 768 dimensions
        assert_eq!(
            embedding.len(),
            768,
            "Embedding should have 768 dimensions for balanced preset"
        );

        // Verify embeddings are normalized
        let magnitude: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!(
            (magnitude - 1.0).abs() < 0.01,
            "Embedding should be normalized (magnitude ~= 1.0)"
        );
    }
}

/// Test chunking with fast embedding preset.
#[tokio::test]
#[cfg(feature = "embeddings")]
async fn test_chunking_with_fast_embeddings() {
    use kreuzberg::core::config::{EmbeddingConfig, EmbeddingModelType};

    let config = ExtractionConfig {
        chunking: Some(ChunkingConfig {
            max_chars: 100,
            max_overlap: 20,
            embedding: Some(EmbeddingConfig {
                model: EmbeddingModelType::Preset {
                    name: "fast".to_string(),
                },
                ..Default::default()
            }),
            preset: None,
        }),
        ..Default::default()
    };

    let text = "Fast embedding test. ".repeat(10);
    let text_bytes = text.as_bytes();

    let result = extract_bytes(text_bytes, "text/plain", &config)
        .await
        .expect("Should extract successfully");

    let chunks = result.chunks.expect("Should have chunks");
    assert!(chunks.len() > 0, "Should have at least one chunk");

    // Verify embeddings have correct dimensions for fast preset (384)
    for chunk in &chunks {
        let embedding = chunk.embedding.as_ref().expect("Should have embedding");
        assert_eq!(embedding.len(), 384, "Fast preset should produce 384-dim embeddings");
    }
}
