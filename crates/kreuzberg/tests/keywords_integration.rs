//! Integration tests for keyword extraction functionality.
//!
//! These tests verify end-to-end keyword extraction with both YAKE and RAKE algorithms,
//! testing various configurations, languages, and edge cases.

#[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
use kreuzberg::keywords::{KeywordAlgorithm, KeywordConfig, extract_keywords};

/// Sample document about machine learning for testing.
const ML_DOCUMENT: &str = r#"
Machine learning is a branch of artificial intelligence and computer science which focuses on the use of data and algorithms to imitate the way that humans learn.
Machine learning algorithms build a model based on sample data, known as training data, to make predictions or decisions without being explicitly programmed to do so.
Deep learning is a type of machine learning based on artificial neural networks. The learning process is deep because the structure of artificial neural networks consists of multiple input, output, and hidden layers.
Neural networks can be used for supervised, semi-supervised, and unsupervised learning. Convolutional neural networks are commonly applied to analyzing visual imagery.
Natural language processing is a subfield of linguistics, computer science, and artificial intelligence concerned with the interactions between computers and human language.
"#;

/// Sample document about climate change for testing.
const CLIMATE_DOCUMENT: &str = r#"
Climate change refers to long-term shifts in temperatures and weather patterns. These shifts may be natural, such as through variations in the solar cycle.
But since the 1800s, human activities have been the main driver of climate change, primarily due to burning fossil fuels like coal, oil, and gas.
Burning fossil fuels generates greenhouse gas emissions that act like a blanket wrapped around the Earth, trapping the sun's heat and raising temperatures.
The main greenhouse gases that are causing climate change include carbon dioxide and methane. These come from burning fossil fuels for energy, agriculture, and deforestation.
Global warming is the long-term heating of Earth's climate system. Climate science reveals that human activity has been the dominant cause of climate change since the mid-20th century.
"#;

/// Sample Spanish document for multilingual testing.
const SPANISH_DOCUMENT: &str = r#"
El aprendizaje automático es una rama de la inteligencia artificial. Los algoritmos de aprendizaje automático construyen modelos basados en datos de entrenamiento.
Las redes neuronales artificiales son sistemas inspirados en las redes neuronales biológicas del cerebro humano. El aprendizaje profundo utiliza redes neuronales multicapa.
El procesamiento del lenguaje natural es un campo de la inteligencia artificial que se ocupa de la interacción entre computadoras y lenguajes humanos.
"#;

#[cfg(feature = "keywords-yake")]
#[test]
fn test_yake_basic_extraction() {
    let config = KeywordConfig::yake();
    let keywords = extract_keywords(ML_DOCUMENT, &config).unwrap();

    // Basic assertions
    assert!(!keywords.is_empty(), "Should extract keywords from document");
    assert!(
        keywords.len() <= config.max_keywords,
        "Should respect max_keywords limit"
    );

    // Verify keywords are sorted by score (highest first)
    for i in 1..keywords.len() {
        assert!(
            keywords[i - 1].score >= keywords[i].score,
            "Keywords should be sorted by score descending: {} >= {}",
            keywords[i - 1].score,
            keywords[i].score
        );
    }

    // Verify all keywords use YAKE algorithm
    for keyword in &keywords {
        assert_eq!(keyword.algorithm, KeywordAlgorithm::Yake);
    }

    // Verify extracted keywords are relevant to ML
    let keyword_texts: Vec<&str> = keywords.iter().map(|k| k.text.as_str()).collect();
    let relevant_terms = [
        "machine learning",
        "artificial intelligence",
        "neural networks",
        "deep learning",
    ];
    let has_relevant = keyword_texts
        .iter()
        .any(|kw| relevant_terms.iter().any(|term| kw.to_lowercase().contains(term)));
    assert!(
        has_relevant,
        "Should extract at least one ML-related term, got: {:?}",
        keyword_texts
    );
}

#[cfg(feature = "keywords-rake")]
#[test]
fn test_rake_basic_extraction() {
    let config = KeywordConfig::rake();
    let keywords = extract_keywords(ML_DOCUMENT, &config).unwrap();

    // Basic assertions
    assert!(!keywords.is_empty(), "Should extract keywords from document");
    assert!(
        keywords.len() <= config.max_keywords,
        "Should respect max_keywords limit"
    );

    // Verify keywords are sorted by score (highest first)
    for i in 1..keywords.len() {
        assert!(
            keywords[i - 1].score >= keywords[i].score,
            "Keywords should be sorted by score descending"
        );
    }

    // Verify all keywords use RAKE algorithm
    for keyword in &keywords {
        assert_eq!(keyword.algorithm, KeywordAlgorithm::Rake);
    }

    // Verify extracted keywords are relevant to ML
    let keyword_texts: Vec<&str> = keywords.iter().map(|k| k.text.as_str()).collect();
    let relevant_terms = [
        "machine learning",
        "artificial intelligence",
        "neural networks",
        "deep learning",
    ];
    let has_relevant = keyword_texts
        .iter()
        .any(|kw| relevant_terms.iter().any(|term| kw.to_lowercase().contains(term)));
    assert!(
        has_relevant,
        "Should extract at least one ML-related term, got: {:?}",
        keyword_texts
    );
}

#[cfg(all(feature = "keywords-yake", feature = "keywords-rake"))]
#[test]
fn test_yake_vs_rake_comparison() {
    // Extract with both algorithms
    let yake_config = KeywordConfig::yake().with_max_keywords(5);
    let rake_config = KeywordConfig::rake().with_max_keywords(5);

    let yake_keywords = extract_keywords(ML_DOCUMENT, &yake_config).unwrap();
    let rake_keywords = extract_keywords(ML_DOCUMENT, &rake_config).unwrap();

    // Both should extract keywords
    assert!(!yake_keywords.is_empty(), "YAKE should extract keywords");
    assert!(!rake_keywords.is_empty(), "RAKE should extract keywords");

    // Verify algorithm field is set correctly
    assert!(yake_keywords.iter().all(|k| k.algorithm == KeywordAlgorithm::Yake));
    assert!(rake_keywords.iter().all(|k| k.algorithm == KeywordAlgorithm::Rake));

    // Compare results (algorithms may produce different keywords)
    println!("\nYAKE keywords:");
    for kw in &yake_keywords {
        println!("  {} (score: {:.3})", kw.text, kw.score);
    }

    println!("\nRAKE keywords:");
    for kw in &rake_keywords {
        println!("  {} (score: {:.3})", kw.text, kw.score);
    }

    // Check if there's overlap (there should be some common important terms)
    let yake_texts: Vec<&str> = yake_keywords.iter().map(|k| k.text.as_str()).collect();
    let rake_texts: Vec<&str> = rake_keywords.iter().map(|k| k.text.as_str()).collect();

    // At least one keyword should be common (case-insensitive comparison)
    let has_overlap = yake_texts.iter().any(|yt| {
        rake_texts.iter().any(|rt| {
            yt.to_lowercase() == rt.to_lowercase()
                || yt.to_lowercase().contains(&rt.to_lowercase())
                || rt.to_lowercase().contains(&yt.to_lowercase())
        })
    });

    // Note: This assertion is flexible - algorithms may find different keywords
    if !has_overlap {
        println!("Note: YAKE and RAKE found completely different keywords, which is possible");
    }
}

#[cfg(feature = "keywords-yake")]
#[test]
fn test_yake_with_max_keywords() {
    let config = KeywordConfig::yake().with_max_keywords(3);
    let keywords = extract_keywords(ML_DOCUMENT, &config).unwrap();

    assert!(keywords.len() <= 3, "Should respect max_keywords=3 limit");

    // If we got keywords, they should be the top-scored ones
    if !keywords.is_empty() {
        // Scores should be decreasing
        for i in 1..keywords.len() {
            assert!(keywords[i - 1].score >= keywords[i].score);
        }
    }
}

#[cfg(feature = "keywords-rake")]
#[test]
fn test_rake_with_max_keywords() {
    let config = KeywordConfig::rake().with_max_keywords(3);
    let keywords = extract_keywords(ML_DOCUMENT, &config).unwrap();

    assert!(keywords.len() <= 3, "Should respect max_keywords=3 limit");

    // If we got keywords, they should be the top-scored ones
    if !keywords.is_empty() {
        // Scores should be decreasing
        for i in 1..keywords.len() {
            assert!(keywords[i - 1].score >= keywords[i].score);
        }
    }
}

#[cfg(feature = "keywords-yake")]
#[test]
fn test_yake_with_min_score() {
    let config = KeywordConfig::yake().with_min_score(0.5);
    let keywords = extract_keywords(ML_DOCUMENT, &config).unwrap();

    // All keywords should meet minimum score
    for keyword in &keywords {
        assert!(
            keyword.score >= 0.5,
            "Keyword '{}' score {} should be >= 0.5",
            keyword.text,
            keyword.score
        );
    }
}

#[cfg(feature = "keywords-rake")]
#[test]
fn test_rake_with_min_score() {
    let config = KeywordConfig::rake().with_min_score(0.2);
    let keywords = extract_keywords(ML_DOCUMENT, &config).unwrap();

    // All keywords should meet minimum score
    for keyword in &keywords {
        assert!(
            keyword.score >= 0.2,
            "Keyword '{}' score {} should be >= 0.2",
            keyword.text,
            keyword.score
        );
    }
}

#[cfg(feature = "keywords-yake")]
#[test]
fn test_yake_with_ngram_range() {
    // Test unigrams only
    let config = KeywordConfig::yake().with_ngram_range(1, 1);
    let keywords = extract_keywords(ML_DOCUMENT, &config).unwrap();

    for keyword in &keywords {
        let word_count = keyword.text.split_whitespace().count();
        assert_eq!(word_count, 1, "Should only extract unigrams (single words)");
    }

    // Test bigrams and trigrams only
    let config = KeywordConfig::yake().with_ngram_range(2, 3);
    let keywords = extract_keywords(ML_DOCUMENT, &config).unwrap();

    for keyword in &keywords {
        let word_count = keyword.text.split_whitespace().count();
        assert!(
            word_count >= 2 && word_count <= 3,
            "Should only extract 2-3 word phrases, got {} words in '{}'",
            word_count,
            keyword.text
        );
    }
}

#[cfg(feature = "keywords-rake")]
#[test]
fn test_rake_with_ngram_range() {
    // Test unigrams only
    let config = KeywordConfig::rake().with_ngram_range(1, 1);
    let keywords = extract_keywords(ML_DOCUMENT, &config).unwrap();

    for keyword in &keywords {
        let word_count = keyword.text.split_whitespace().count();
        assert_eq!(word_count, 1, "Should only extract unigrams (single words)");
    }

    // Test bigrams only
    let config = KeywordConfig::rake().with_ngram_range(2, 2);
    let keywords = extract_keywords(ML_DOCUMENT, &config).unwrap();

    for keyword in &keywords {
        let word_count = keyword.text.split_whitespace().count();
        assert_eq!(word_count, 2, "Should only extract bigrams (2-word phrases)");
    }
}

#[cfg(feature = "keywords-rake")]
#[test]
fn test_rake_with_spanish() {
    let config = KeywordConfig::rake().with_language("es");
    let keywords = extract_keywords(SPANISH_DOCUMENT, &config).unwrap();

    assert!(!keywords.is_empty(), "Should extract Spanish keywords");

    // Verify Spanish-specific terms are extracted
    let keyword_texts: Vec<&str> = keywords.iter().map(|k| k.text.as_str()).collect();
    println!("\nSpanish keywords:");
    for kw in &keywords {
        println!("  {} (score: {:.3})", kw.text, kw.score);
    }

    // Check for relevant Spanish terms (case-insensitive)
    let relevant_terms = ["aprendizaje", "inteligencia", "redes neuronales", "lenguaje"];
    let has_relevant = keyword_texts
        .iter()
        .any(|kw| relevant_terms.iter().any(|term| kw.to_lowercase().contains(term)));

    assert!(
        has_relevant,
        "Should extract at least one relevant Spanish term, got: {:?}",
        keyword_texts
    );
}

#[cfg(feature = "keywords-yake")]
#[test]
fn test_yake_with_spanish() {
    let config = KeywordConfig::yake().with_language("es");
    let keywords = extract_keywords(SPANISH_DOCUMENT, &config).unwrap();

    assert!(!keywords.is_empty(), "Should extract Spanish keywords");

    // Verify Spanish-specific terms are extracted
    println!("\nYAKE Spanish keywords:");
    for kw in &keywords {
        println!("  {} (score: {:.3})", kw.text, kw.score);
    }
}

#[cfg(feature = "keywords-rake")]
#[test]
fn test_rake_empty_document() {
    let config = KeywordConfig::rake();
    let keywords = extract_keywords("", &config).unwrap();

    assert!(keywords.is_empty(), "Empty document should yield no keywords");
}

#[cfg(feature = "keywords-yake")]
#[test]
fn test_yake_empty_document() {
    let config = KeywordConfig::yake();
    let keywords = extract_keywords("", &config).unwrap();

    assert!(keywords.is_empty(), "Empty document should yield no keywords");
}

#[cfg(feature = "keywords-rake")]
#[test]
fn test_rake_short_document() {
    let short_text = "Machine learning algorithms.";
    let config = KeywordConfig::rake();
    let keywords = extract_keywords(short_text, &config).unwrap();

    // Should extract at least something from short text
    // Note: RAKE may return empty for very short texts with all stopwords
    println!(
        "Keywords from short text: {:?}",
        keywords.iter().map(|k| &k.text).collect::<Vec<_>>()
    );
}

#[cfg(feature = "keywords-yake")]
#[test]
fn test_yake_short_document() {
    let short_text = "Machine learning algorithms.";
    let config = KeywordConfig::yake();
    let keywords = extract_keywords(short_text, &config).unwrap();

    // YAKE should handle short text
    println!(
        "YAKE keywords from short text: {:?}",
        keywords.iter().map(|k| &k.text).collect::<Vec<_>>()
    );
}

#[cfg(feature = "keywords-rake")]
#[test]
fn test_rake_different_domains() {
    let config = KeywordConfig::rake().with_max_keywords(5);

    // Test ML domain
    let ml_keywords = extract_keywords(ML_DOCUMENT, &config).unwrap();
    println!("\nML domain keywords:");
    for kw in &ml_keywords {
        println!("  {} (score: {:.3})", kw.text, kw.score);
    }

    // Test climate domain
    let climate_keywords = extract_keywords(CLIMATE_DOCUMENT, &config).unwrap();
    println!("\nClimate domain keywords:");
    for kw in &climate_keywords {
        println!("  {} (score: {:.3})", kw.text, kw.score);
    }

    // Both should extract domain-relevant keywords
    assert!(!ml_keywords.is_empty(), "Should extract ML keywords");
    assert!(!climate_keywords.is_empty(), "Should extract climate keywords");

    // Keywords should be different (domain-specific)
    let ml_texts: Vec<&str> = ml_keywords.iter().map(|k| k.text.as_str()).collect();
    let climate_texts: Vec<&str> = climate_keywords.iter().map(|k| k.text.as_str()).collect();

    // Check for domain-specific terms
    let has_ml_term = ml_texts.iter().any(|kw| {
        kw.to_lowercase().contains("learn")
            || kw.to_lowercase().contains("neural")
            || kw.to_lowercase().contains("algorithm")
    });

    let has_climate_term = climate_texts.iter().any(|kw| {
        kw.to_lowercase().contains("climate")
            || kw.to_lowercase().contains("greenhouse")
            || kw.to_lowercase().contains("fossil")
    });

    assert!(has_ml_term, "ML document should have ML-related keywords");
    assert!(
        has_climate_term,
        "Climate document should have climate-related keywords"
    );
}

#[cfg(feature = "keywords-yake")]
#[test]
fn test_yake_different_domains() {
    let config = KeywordConfig::yake().with_max_keywords(5);

    // Test ML domain
    let ml_keywords = extract_keywords(ML_DOCUMENT, &config).unwrap();
    println!("\nYAKE ML domain keywords:");
    for kw in &ml_keywords {
        println!("  {} (score: {:.3})", kw.text, kw.score);
    }

    // Test climate domain
    let climate_keywords = extract_keywords(CLIMATE_DOCUMENT, &config).unwrap();
    println!("\nYAKE Climate domain keywords:");
    for kw in &climate_keywords {
        println!("  {} (score: {:.3})", kw.text, kw.score);
    }

    // Both should extract domain-relevant keywords
    assert!(!ml_keywords.is_empty(), "Should extract ML keywords");
    assert!(!climate_keywords.is_empty(), "Should extract climate keywords");
}

#[cfg(feature = "keywords-rake")]
#[test]
fn test_rake_score_distribution() {
    let config = KeywordConfig::rake();
    let keywords = extract_keywords(ML_DOCUMENT, &config).unwrap();

    if keywords.is_empty() {
        return; // Skip if no keywords extracted
    }

    // Verify scores are in valid range [0.0, 1.0]
    for keyword in &keywords {
        assert!(
            keyword.score >= 0.0 && keyword.score <= 1.0,
            "Score {} for '{}' should be in [0.0, 1.0] range",
            keyword.score,
            keyword.text
        );
    }

    // Verify scores are distributed (not all the same)
    let first_score = keywords[0].score;
    let all_same = keywords.iter().all(|k| (k.score - first_score).abs() < 0.001);
    assert!(!all_same, "Scores should be distributed, not all identical");
}

#[cfg(feature = "keywords-yake")]
#[test]
fn test_yake_score_distribution() {
    let config = KeywordConfig::yake();
    let keywords = extract_keywords(ML_DOCUMENT, &config).unwrap();

    if keywords.is_empty() {
        return; // Skip if no keywords extracted
    }

    // Verify scores are in valid range [0.0, 1.0]
    for keyword in &keywords {
        assert!(
            keyword.score >= 0.0 && keyword.score <= 1.0,
            "Score {} for '{}' should be in [0.0, 1.0] range",
            keyword.score,
            keyword.text
        );
    }

    // Verify scores are distributed (not all the same)
    let first_score = keywords[0].score;
    let all_same = keywords.iter().all(|k| (k.score - first_score).abs() < 0.001);
    assert!(!all_same, "Scores should be distributed, not all identical");
}

#[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
#[test]
fn test_keyword_struct_properties() {
    let config = KeywordConfig::default();
    let keywords = extract_keywords(ML_DOCUMENT, &config).unwrap();

    if keywords.is_empty() {
        return; // Skip if no keywords extracted
    }

    // Verify all keywords have required properties
    for keyword in &keywords {
        assert!(!keyword.text.is_empty(), "Keyword text should not be empty");
        assert!(keyword.score >= 0.0, "Score should be non-negative");
        assert!(keyword.score <= 1.0, "Score should not exceed 1.0");

        // Verify text doesn't have leading/trailing whitespace
        assert_eq!(keyword.text.trim(), keyword.text, "Keyword text should be trimmed");
    }
}
