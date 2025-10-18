//! RAKE (Rapid Automatic Keyword Extraction) backend implementation.

use super::config::KeywordConfig;
use super::types::{Keyword, KeywordAlgorithm};
use crate::Result;
use crate::stopwords::STOPWORDS;
use rake::*;

/// Extract keywords using RAKE algorithm.
///
/// RAKE is a co-occurrence based keyword extraction method that:
/// - Identifies candidate keywords using stop words as delimiters
/// - Calculates word scores based on frequency and degree
/// - Combines scores for multi-word phrases
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
pub fn extract_keywords_rake(text: &str, config: &KeywordConfig) -> Result<Vec<Keyword>> {
    // Get RAKE-specific parameters
    let params = config.rake_params.as_ref().cloned().unwrap_or_default();

    // Get stopwords from the stopwords module
    let stopwords = {
        let lang = config.language.as_deref().unwrap_or("en");
        let words: std::collections::HashSet<String> = STOPWORDS
            .get(lang)
            .or_else(|| STOPWORDS.get("en"))
            .map(|set| set.iter().cloned().collect())
            .unwrap_or_default();
        StopWords::from(words)
    };

    // Create RAKE instance
    let rake = Rake::new(stopwords);

    // Extract keywords
    let results = rake.run(text);

    // First pass: collect filtered results with raw scores
    let filtered_results: Vec<_> = results
        .into_iter()
        .filter_map(|keyword_score| {
            let keyword = keyword_score.keyword.clone();

            // Apply min word length filter
            if keyword.len() < params.min_word_length {
                return None;
            }

            // Apply max words per phrase filter
            let word_count = keyword.split_whitespace().count();
            if word_count > params.max_words_per_phrase {
                return None;
            }

            // Apply n-gram range filter
            if word_count < config.ngram_range.0 || word_count > config.ngram_range.1 {
                return None;
            }

            Some((keyword, keyword_score.score))
        })
        .collect();

    // Find min and max scores for normalization
    let min_score = filtered_results.iter().map(|(_, s)| *s).fold(f64::INFINITY, f64::min);
    let max_score = filtered_results
        .iter()
        .map(|(_, s)| *s)
        .fold(f64::NEG_INFINITY, f64::max);

    // Normalize scores using min-max scaling to 0.0-1.0 range
    let mut keywords: Vec<_> = filtered_results
        .into_iter()
        .map(|(keyword, raw_score)| {
            let normalized_score = if max_score > min_score {
                // Min-max normalization: (score - min) / (max - min)
                ((raw_score - min_score) / (max_score - min_score)).clamp(0.0, 1.0)
            } else {
                // All scores are the same, assign 1.0
                1.0
            };

            Keyword::new(keyword, normalized_score as f32, KeywordAlgorithm::Rake)
        })
        .collect();

    // Filter by minimum score
    if config.min_score > 0.0 {
        keywords.retain(|k| k.score >= config.min_score);
    }

    // Sort by score (highest first)
    keywords.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

    // Limit to max_keywords
    keywords.truncate(config.max_keywords);

    Ok(keywords)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keywords::config::RakeParams;

    #[test]
    fn test_rake_extraction_basic() {
        let text = "Rust is a systems programming language. \
                    Rust provides memory safety and performance. \
                    Memory safety is achieved without garbage collection.";

        let config = KeywordConfig::rake();

        let keywords = extract_keywords_rake(text, &config).unwrap();

        assert!(!keywords.is_empty(), "Should extract keywords");
        assert!(
            keywords.len() <= config.max_keywords,
            "Should respect max_keywords limit"
        );

        // Verify keywords are sorted by score
        for i in 1..keywords.len() {
            assert!(
                keywords[i - 1].score >= keywords[i].score,
                "Keywords should be sorted by score"
            );
        }

        // Verify algorithm field
        for keyword in &keywords {
            assert_eq!(keyword.algorithm, KeywordAlgorithm::Rake);
        }
    }

    #[test]
    fn test_rake_extraction_with_min_score() {
        let text = "Rust programming language provides memory safety without garbage collection.";

        let config = KeywordConfig::rake().with_min_score(0.3);

        let keywords = extract_keywords_rake(text, &config).unwrap();

        // Verify all keywords meet minimum score
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
    fn test_rake_extraction_with_ngram_range() {
        let text = "Machine learning models require large datasets for training.";

        // Unigrams only
        let config = KeywordConfig::rake().with_ngram_range(1, 1);
        let keywords = extract_keywords_rake(text, &config).unwrap();

        // All keywords should be single words
        for keyword in &keywords {
            assert_eq!(
                keyword.text.split_whitespace().count(),
                1,
                "Should only extract unigrams"
            );
        }
    }

    #[test]
    fn test_rake_extraction_empty_text() {
        let config = KeywordConfig::rake();
        let keywords = extract_keywords_rake("", &config).unwrap();
        assert!(keywords.is_empty(), "Empty text should yield no keywords");
    }

    #[test]
    fn test_rake_extraction_with_custom_params() {
        let text = "Natural language processing enables computers to understand human language.";

        let params = RakeParams {
            min_word_length: 3,
            max_words_per_phrase: 2,
        };

        let config = KeywordConfig::rake().with_rake_params(params);

        let keywords = extract_keywords_rake(text, &config).unwrap();

        // Verify min word length
        for keyword in &keywords {
            for word in keyword.text.split_whitespace() {
                assert!(word.len() >= 3, "Word '{}' should have min length 3", word);
            }
        }

        // Verify max words per phrase
        for keyword in &keywords {
            assert!(
                keyword.text.split_whitespace().count() <= 2,
                "Keyword '{}' should have max 2 words",
                keyword.text
            );
        }
    }

    #[test]
    fn test_rake_multilingual() {
        // Spanish text (we have Spanish stopwords)
        let spanish_text = "El idioma español es una lengua romance.";
        let config = KeywordConfig::rake().with_language("es");
        let keywords = extract_keywords_rake(spanish_text, &config).unwrap();
        assert!(!keywords.is_empty(), "Should extract Spanish keywords");

        // Verify Spanish keywords are extracted
        assert!(
            keywords
                .iter()
                .any(|k| k.text.contains("idioma") || k.text.contains("español") || k.text.contains("lengua"))
        );

        // Unsupported language falls back to English stopwords
        let english_text = "Natural language processing is a subfield of artificial intelligence.";
        let config = KeywordConfig::rake().with_language("fr"); // French not supported
        let keywords = extract_keywords_rake(english_text, &config).unwrap();
        assert!(
            !keywords.is_empty(),
            "Should fall back to English stopwords and extract keywords"
        );
    }

    #[test]
    fn test_rake_score_normalization() {
        let text = "Rust is a systems programming language that provides memory safety and \
                    thread safety without garbage collection. Rust uses a ownership system \
                    with rules that the compiler checks at compile time.";

        let config = KeywordConfig::rake();
        let keywords = extract_keywords_rake(text, &config).unwrap();

        assert!(!keywords.is_empty(), "Should extract keywords");

        // Verify all scores are in 0.0-1.0 range
        for keyword in &keywords {
            assert!(
                keyword.score >= 0.0 && keyword.score <= 1.0,
                "Keyword '{}' score {} should be in range [0.0, 1.0]",
                keyword.text,
                keyword.score
            );
        }

        // Verify highest score is 1.0 (due to min-max normalization)
        if !keywords.is_empty() {
            let max_score = keywords.iter().map(|k| k.score).fold(0.0f32, f32::max);
            assert!(
                (max_score - 1.0).abs() < 0.001,
                "Max score should be 1.0, got {}",
                max_score
            );
        }

        // Verify scores are sorted (highest first)
        for i in 1..keywords.len() {
            assert!(
                keywords[i - 1].score >= keywords[i].score,
                "Keywords should be sorted by score"
            );
        }
    }
}
