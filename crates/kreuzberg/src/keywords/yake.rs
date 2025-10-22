//! YAKE (Yet Another Keyword Extractor) backend implementation.

use super::config::KeywordConfig;
use super::types::{Keyword, KeywordAlgorithm};
use crate::Result;
use yake_rust::{Config as YakeConfig, StopWords, get_n_best};

/// Extract keywords using YAKE algorithm.
///
/// YAKE is a statistical keyword extraction method that weighs multiple factors:
/// - Term frequency and position
/// - Capitalization
/// - Sentence co-occurrence
/// - Context analysis
///
/// # Arguments
///
/// * `text` - The text to extract keywords from
/// * `config` - Keyword extraction configuration
///
/// # Returns
///
/// A vector of keywords sorted by relevance (highest score first).
///
/// # Errors
///
/// Returns an error if keyword extraction fails.
pub fn extract_keywords_yake(text: &str, config: &KeywordConfig) -> Result<Vec<Keyword>> {
    let params = config.yake_params.as_ref().cloned().unwrap_or_default();

    let yake_config = YakeConfig {
        ngrams: config.ngram_range.1,
        window_size: params.window_size,
        ..YakeConfig::default()
    };

    let stopwords = if let Some(ref lang) = config.language {
        StopWords::predefined(lang).unwrap_or_else(|| {
            eprintln!(
                "WARNING: Stopwords not available for language '{}', using default English stopwords",
                lang
            );
            StopWords::default()
        })
    } else {
        StopWords::default()
    };

    let results = get_n_best(config.max_keywords, text, &stopwords, &yake_config);

    let mut keywords = results
        .into_iter()
        .filter(|item| {
            let word_count = item.keyword.split_whitespace().count();
            word_count >= config.ngram_range.0
        })
        .map(|item| {
            let normalized_score = if item.score > 0.0 {
                (1.0 / (1.0 + item.score)).clamp(0.0, 1.0)
            } else {
                1.0
            };

            Keyword::new(item.keyword, normalized_score as f32, KeywordAlgorithm::Yake)
        })
        .collect::<Vec<_>>();

    if config.min_score > 0.0 {
        keywords.retain(|k| k.score >= config.min_score);
    }

    keywords.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

    Ok(keywords)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keywords::config::YakeParams;

    #[test]
    fn test_yake_extraction_basic() {
        let text = "Rust is a systems programming language. \
                    Rust provides memory safety and performance. \
                    Memory safety is achieved without garbage collection.";

        let config = KeywordConfig::yake();

        let keywords = extract_keywords_yake(text, &config).unwrap();

        assert!(!keywords.is_empty(), "Should extract keywords");
        assert!(
            keywords.len() <= config.max_keywords,
            "Should respect max_keywords limit"
        );

        for i in 1..keywords.len() {
            assert!(
                keywords[i - 1].score >= keywords[i].score,
                "Keywords should be sorted by score"
            );
        }

        for keyword in &keywords {
            assert_eq!(keyword.algorithm, KeywordAlgorithm::Yake);
        }
    }

    #[test]
    fn test_yake_extraction_with_min_score() {
        let text = "Rust programming language provides memory safety without garbage collection.";

        let config = KeywordConfig::yake().with_min_score(0.5);

        let keywords = extract_keywords_yake(text, &config).unwrap();

        for keyword in &keywords {
            assert!(
                keyword.score >= config.min_score,
                "Keyword score {} should be >= min_score {}",
                keyword.score,
                config.min_score
            );
        }
    }

    #[test]
    fn test_yake_extraction_with_ngram_range() {
        let text = "Machine learning models require large datasets for training.";

        let config = KeywordConfig::yake().with_ngram_range(1, 1);
        let keywords = extract_keywords_yake(text, &config).unwrap();

        for keyword in &keywords {
            assert_eq!(
                keyword.text.split_whitespace().count(),
                1,
                "Should only extract unigrams"
            );
        }
    }

    #[test]
    fn test_yake_extraction_empty_text() {
        let config = KeywordConfig::yake();
        let keywords = extract_keywords_yake("", &config).unwrap();
        assert!(keywords.is_empty(), "Empty text should yield no keywords");
    }

    #[test]
    fn test_yake_extraction_with_custom_params() {
        let text = "Natural language processing enables computers to understand human language.";

        let params = YakeParams { window_size: 3 };

        let config = KeywordConfig::yake().with_yake_params(params);

        let keywords = extract_keywords_yake(text, &config).unwrap();

        assert!(!keywords.is_empty(), "Should extract keywords with custom params");
    }
}
