//! Integration tests for stopwords with token reduction and keywords extraction.
//!
//! These tests verify that stopwords are properly integrated across different features:
//! - Token reduction at all ReductionLevels
//! - Keywords extraction (YAKE and RAKE algorithms)
//! - CJK text processing
//! - Multi-language documents
//! - Language fallback mechanisms
//! - Custom stopwords

use kreuzberg::stopwords::{STOPWORDS, get_stopwords, get_stopwords_with_fallback};
use kreuzberg::text::token_reduction::{ReductionLevel, TokenReductionConfig, reduce_tokens};

#[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
use kreuzberg::keywords::{KeywordConfig, extract_keywords};

use std::collections::HashMap;

fn count_stopwords(text: &str, lang: &str) -> usize {
    let stopwords = get_stopwords(lang).expect("Stopwords must exist for language");
    let words: Vec<&str> = text.split_whitespace().collect();

    words
        .iter()
        .filter(|word| {
            let clean = word
                .chars()
                .filter(|c| c.is_alphabetic())
                .collect::<String>()
                .to_lowercase();

            !clean.is_empty() && stopwords.contains(&clean)
        })
        .count()
}

fn extract_content_words(text: &str, lang: &str) -> Vec<String> {
    let stopwords = get_stopwords(lang).expect("Stopwords must exist for language");
    let words: Vec<&str> = text.split_whitespace().collect();

    words
        .iter()
        .filter_map(|word| {
            let clean = word
                .chars()
                .filter(|c| c.is_alphabetic())
                .collect::<String>()
                .to_lowercase();

            if !clean.is_empty() && !stopwords.contains(&clean) && clean.len() > 1 {
                Some(clean)
            } else {
                None
            }
        })
        .collect()
}

#[test]
fn test_stopwords_removed_during_moderate_token_reduction() {
    let config = TokenReductionConfig {
        level: ReductionLevel::Moderate,
        language_hint: Some("en".to_string()),
        use_simd: false,
        ..Default::default()
    };

    let input = "The quick brown fox is jumping over the lazy dog and running through the forest";
    let result = reduce_tokens(input, &config, Some("en")).unwrap();

    assert!(!result.contains(" the "), "Should remove 'the'. Result: {}", result);
    assert!(!result.contains(" is "), "Should remove 'is'. Result: {}", result);
    assert!(!result.contains(" and "), "Should remove 'and'. Result: {}", result);

    assert!(result.contains("quick"), "Should preserve 'quick'. Result: {}", result);
    assert!(result.contains("brown"), "Should preserve 'brown'. Result: {}", result);
    assert!(result.contains("fox"), "Should preserve 'fox'. Result: {}", result);
    assert!(
        result.contains("jumping"),
        "Should preserve 'jumping'. Result: {}",
        result
    );
    assert!(result.contains("lazy"), "Should preserve 'lazy'. Result: {}", result);

    let original_stopwords = count_stopwords(input, "en");
    let result_stopwords = count_stopwords(&result, "en");

    assert!(
        result_stopwords < original_stopwords,
        "Result should have fewer stopwords than original. Original: {}, Result: {}",
        original_stopwords,
        result_stopwords
    );
}

#[test]
fn test_stopwords_across_reduction_levels() {
    let text = "The machine learning model is trained on the large dataset and achieves good performance";

    let light_config = TokenReductionConfig {
        level: ReductionLevel::Light,
        use_simd: false,
        ..Default::default()
    };
    let light_result = reduce_tokens(text, &light_config, Some("en")).unwrap();

    let light_stopwords = count_stopwords(&light_result, "en");
    assert!(light_stopwords > 0, "Light reduction should preserve some stopwords");

    let moderate_config = TokenReductionConfig {
        level: ReductionLevel::Moderate,
        use_simd: false,
        ..Default::default()
    };
    let moderate_result = reduce_tokens(text, &moderate_config, Some("en")).unwrap();

    let moderate_stopwords = count_stopwords(&moderate_result, "en");
    assert!(
        moderate_stopwords < light_stopwords,
        "Moderate reduction should remove more stopwords than light. Light: {}, Moderate: {}",
        light_stopwords,
        moderate_stopwords
    );

    let aggressive_config = TokenReductionConfig {
        level: ReductionLevel::Aggressive,
        use_simd: false,
        ..Default::default()
    };
    let aggressive_result = reduce_tokens(text, &aggressive_config, Some("en")).unwrap();

    assert!(
        aggressive_result.len() <= moderate_result.len(),
        "Aggressive reduction should be more aggressive than moderate"
    );
}

#[test]
fn test_stopwords_preserve_semantic_meaning() {
    let config = TokenReductionConfig {
        level: ReductionLevel::Moderate,
        use_simd: false,
        ..Default::default()
    };

    let input =
        "The artificial intelligence system is processing the natural language text for extracting meaningful insights";
    let result = reduce_tokens(input, &config, Some("en")).unwrap();

    let content_words = extract_content_words(&result, "en");

    assert!(
        content_words.contains(&"artificial".to_string()) || result.contains("artificial"),
        "Should preserve 'artificial'. Result: {}",
        result
    );
    assert!(
        content_words.contains(&"intelligence".to_string()) || result.contains("intelligence"),
        "Should preserve 'intelligence'. Result: {}",
        result
    );
    assert!(
        content_words.contains(&"processing".to_string()) || result.contains("processing"),
        "Should preserve 'processing'. Result: {}",
        result
    );
    assert!(
        content_words.contains(&"natural".to_string()) || result.contains("natural"),
        "Should preserve 'natural'. Result: {}",
        result
    );
    assert!(
        content_words.contains(&"language".to_string()) || result.contains("language"),
        "Should preserve 'language'. Result: {}",
        result
    );
}

#[test]
fn test_stopwords_with_multiple_languages() {
    let en_config = TokenReductionConfig {
        level: ReductionLevel::Moderate,
        use_simd: false,
        ..Default::default()
    };
    let en_input = "The computer science program is very comprehensive and includes many courses";
    let en_result = reduce_tokens(en_input, &en_config, Some("en")).unwrap();

    let en_original_stopwords = count_stopwords(en_input, "en");
    let en_result_stopwords = count_stopwords(&en_result, "en");
    assert!(
        en_result_stopwords < en_original_stopwords,
        "English stopwords should be removed"
    );

    let es_config = TokenReductionConfig {
        level: ReductionLevel::Moderate,
        use_simd: false,
        ..Default::default()
    };
    let es_input = "El programa de ciencias de la computación es muy completo y tiene muchos cursos";
    let es_result = reduce_tokens(es_input, &es_config, Some("es")).unwrap();

    let es_original_stopwords = count_stopwords(es_input, "es");
    let es_result_stopwords = count_stopwords(&es_result, "es");
    assert!(
        es_result_stopwords < es_original_stopwords,
        "Spanish stopwords should be removed"
    );

    assert!(
        es_result.contains("programa") || es_result.contains("ciencias") || es_result.contains("computación"),
        "Should preserve Spanish content words. Result: {}",
        es_result
    );

    let de_config = TokenReductionConfig {
        level: ReductionLevel::Moderate,
        use_simd: false,
        ..Default::default()
    };
    let de_input = "Die künstliche Intelligenz ist ein wichtiges Forschungsgebiet der Informatik";
    let de_result = reduce_tokens(de_input, &de_config, Some("de")).unwrap();

    let de_original_stopwords = count_stopwords(de_input, "de");
    let de_result_stopwords = count_stopwords(&de_result, "de");
    assert!(
        de_result_stopwords < de_original_stopwords,
        "German stopwords should be removed"
    );
}

#[test]
fn test_language_fallback_to_english_stopwords() {
    let config = TokenReductionConfig {
        level: ReductionLevel::Moderate,
        use_simd: false,
        ..Default::default()
    };

    let input = "The system is processing the data with the algorithm";
    let result = reduce_tokens(input, &config, Some("xyz")).unwrap();

    let original_stopwords = count_stopwords(input, "en");
    let result_stopwords = count_stopwords(&result, "en");

    assert!(
        result_stopwords < original_stopwords,
        "Should fallback to English stopwords for unsupported language"
    );
}

#[test]
fn test_custom_stopwords_integration() {
    let mut custom_stopwords = HashMap::new();
    custom_stopwords.insert(
        "en".to_string(),
        vec!["algorithm".to_string(), "system".to_string(), "data".to_string()],
    );

    let config = TokenReductionConfig {
        level: ReductionLevel::Moderate,
        use_simd: false,
        custom_stopwords: Some(custom_stopwords),
        ..Default::default()
    };

    let input = "The algorithm processes the data in the system efficiently";
    let result = reduce_tokens(input, &config, Some("en")).unwrap();

    assert!(
        !result.contains("algorithm"),
        "Should remove custom stopword 'algorithm'. Result: {}",
        result
    );
    assert!(
        !result.contains("system"),
        "Should remove custom stopword 'system'. Result: {}",
        result
    );
    assert!(
        !result.contains("data"),
        "Should remove custom stopword 'data'. Result: {}",
        result
    );

    assert!(
        result.contains("processes") || result.contains("efficiently"),
        "Should preserve non-stopword content. Result: {}",
        result
    );
}

#[test]
fn test_stopwords_with_chinese_text() {
    let config = TokenReductionConfig {
        level: ReductionLevel::Moderate,
        use_simd: false,
        ..Default::default()
    };

    let input = "这个人工智能系统可以处理自然语言";
    let result = reduce_tokens(input, &config, Some("zh")).unwrap();

    assert!(
        !result.is_empty(),
        "Chinese text should be processed. Result: {}",
        result
    );

    assert!(
        result.contains("人工") || result.contains("智能") || result.contains("语言"),
        "Should preserve important Chinese terms. Result: {}",
        result
    );
}

#[test]
fn test_stopwords_with_mixed_cjk_english() {
    let config = TokenReductionConfig {
        level: ReductionLevel::Moderate,
        use_simd: false,
        ..Default::default()
    };

    let input = "The machine learning model 机器学习模型 is processing data efficiently";
    let result = reduce_tokens(input, &config, Some("en")).unwrap();

    assert!(
        !result.contains(" the ") && !result.contains("The "),
        "Should remove English 'the'. Result: {}",
        result
    );

    assert!(
        result.contains("machine") || result.contains("learning"),
        "Should preserve English content. Result: {}",
        result
    );

    assert!(
        result.contains("机器") || result.contains("学习") || result.contains("模型"),
        "Should preserve Chinese content. Result: {}",
        result
    );
}

#[test]
fn test_stopwords_with_japanese_text() {
    let config = TokenReductionConfig {
        level: ReductionLevel::Moderate,
        use_simd: false,
        ..Default::default()
    };

    let input = "人工知能技術の研究開発";
    let result = reduce_tokens(input, &config, Some("ja")).unwrap();

    assert!(
        !result.is_empty(),
        "Japanese text should be processed. Result: {}",
        result
    );
}

#[test]
fn test_stopwords_with_korean_text() {
    let config = TokenReductionConfig {
        level: ReductionLevel::Moderate,
        use_simd: false,
        ..Default::default()
    };

    let input = "인공 지능 기술 개발";
    let result = reduce_tokens(input, &config, Some("ko")).unwrap();

    assert!(
        !result.is_empty(),
        "Korean text should be processed. Result: {}",
        result
    );
}

#[cfg(feature = "keywords-rake")]
#[test]
fn test_stopwords_excluded_from_rake_keywords() {
    let text = "The machine learning model is trained on a large dataset. \
                The model uses neural networks and deep learning algorithms. \
                The training process requires significant computational resources.";

    let config = KeywordConfig::rake().with_language("en").with_max_keywords(10);

    let keywords = extract_keywords(text, &config).unwrap();

    assert!(!keywords.is_empty(), "Should extract keywords");

    let en_stopwords = get_stopwords("en").expect("English stopwords must exist");

    for keyword in &keywords {
        let words: Vec<&str> = keyword.text.split_whitespace().collect();

        let all_stopwords = words.iter().all(|word| {
            let clean = word
                .chars()
                .filter(|c| c.is_alphabetic())
                .collect::<String>()
                .to_lowercase();
            en_stopwords.contains(&clean)
        });

        assert!(
            !all_stopwords,
            "Keyword '{}' should not be composed entirely of stopwords",
            keyword.text
        );
    }

    let keyword_texts: Vec<String> = keywords.iter().map(|k| k.text.to_lowercase()).collect();

    assert!(
        keyword_texts.iter().any(|k| k.contains("machine learning")
            || k.contains("neural networks")
            || k.contains("deep learning")
            || k.contains("dataset")
            || k.contains("model")
            || k.contains("training")),
        "Should extract meaningful technical keywords. Got: {:?}",
        keyword_texts
    );
}

#[cfg(feature = "keywords-yake")]
#[test]
fn test_stopwords_excluded_from_yake_keywords() {
    let text = "Natural language processing enables computers to understand human language. \
                Deep learning models achieve state-of-the-art performance in text analysis. \
                These systems can extract meaningful information from large text corpora.";

    let config = KeywordConfig::yake().with_language("en").with_max_keywords(10);

    let keywords = extract_keywords(text, &config).unwrap();

    assert!(!keywords.is_empty(), "Should extract keywords");

    let en_stopwords = get_stopwords("en").expect("English stopwords must exist");

    for keyword in &keywords {
        let has_content_word = keyword.text.split_whitespace().any(|word| {
            let clean = word
                .chars()
                .filter(|c| c.is_alphabetic())
                .collect::<String>()
                .to_lowercase();
            !clean.is_empty() && !en_stopwords.contains(&clean)
        });

        assert!(
            has_content_word,
            "Keyword '{}' should contain at least one content word (non-stopword)",
            keyword.text
        );
    }
}

#[cfg(feature = "keywords-rake")]
#[test]
fn test_keywords_respect_language_specific_stopwords() {
    let spanish_text = "El aprendizaje automático es una rama de la inteligencia artificial. \
                        Los modelos de aprendizaje profundo logran un rendimiento excepcional. \
                        Estos sistemas pueden procesar grandes cantidades de datos.";

    let config = KeywordConfig::rake().with_language("es").with_max_keywords(8);

    let keywords = extract_keywords(spanish_text, &config).unwrap();

    assert!(!keywords.is_empty(), "Should extract Spanish keywords");

    let es_stopwords = get_stopwords("es").expect("Spanish stopwords must exist");

    for keyword in &keywords {
        let words: Vec<&str> = keyword.text.split_whitespace().collect();
        let all_stopwords = words.iter().all(|word| {
            let clean = word
                .chars()
                .filter(|c| c.is_alphabetic())
                .collect::<String>()
                .to_lowercase();
            es_stopwords.contains(&clean)
        });

        assert!(
            !all_stopwords,
            "Spanish keyword '{}' should not be all stopwords",
            keyword.text
        );
    }

    let keyword_texts: Vec<String> = keywords.iter().map(|k| k.text.to_lowercase()).collect();
    assert!(
        keyword_texts.iter().any(|k| k.contains("aprendizaje")
            || k.contains("inteligencia")
            || k.contains("modelos")
            || k.contains("datos")),
        "Should extract meaningful Spanish keywords. Got: {:?}",
        keyword_texts
    );
}

#[test]
fn test_all_stopwords_text_reduction() {
    let config = TokenReductionConfig {
        level: ReductionLevel::Moderate,
        use_simd: false,
        ..Default::default()
    };

    let input = "the is a an and or but of to in for on at by";
    let result = reduce_tokens(input, &config, Some("en")).unwrap();

    assert!(
        result.len() < input.len(),
        "Text of all stopwords should be significantly reduced"
    );
}

#[test]
fn test_no_stopwords_text_reduction() {
    let config = TokenReductionConfig {
        level: ReductionLevel::Moderate,
        use_simd: false,
        ..Default::default()
    };

    let input = "PyTorch TensorFlow CUDA GPU optimization benchmark performance metrics";
    let result = reduce_tokens(input, &config, Some("en")).unwrap();

    let input_words: Vec<&str> = input.split_whitespace().collect();
    let result_lower = result.to_lowercase();

    for word in input_words {
        let word_lower = word.to_lowercase();
        assert!(
            result_lower.contains(&word_lower),
            "Technical term '{}' should be preserved. Result: {}",
            word,
            result
        );
    }
}

#[test]
fn test_mixed_case_stopwords_removal() {
    let config = TokenReductionConfig {
        level: ReductionLevel::Moderate,
        use_simd: false,
        ..Default::default()
    };

    let input = "The SYSTEM Is Processing The DATA With The ALGORITHM";
    let result = reduce_tokens(input, &config, Some("en")).unwrap();

    let result_words: Vec<&str> = result.split_whitespace().collect();
    assert!(
        !result_words.contains(&"the"),
        "Should remove lowercase 'the'. Result: {}",
        result
    );
    assert!(
        !result_words.contains(&"is"),
        "Should remove lowercase 'is'. Result: {}",
        result
    );

    assert!(
        result.contains("SYSTEM"),
        "Should preserve 'SYSTEM'. Result: {}",
        result
    );
    assert!(result.contains("DATA"), "Should preserve 'DATA'. Result: {}", result);
    assert!(
        result.contains("ALGORITHM"),
        "Should preserve 'ALGORITHM'. Result: {}",
        result
    );
}

#[test]
fn test_reduce_tokens_function_with_stopwords() {
    let config = TokenReductionConfig {
        level: ReductionLevel::Moderate,
        use_simd: false,
        ..Default::default()
    };

    let text = "The artificial intelligence system processes the natural language efficiently";
    let result = reduce_tokens(text, &config, Some("en")).unwrap();

    let original_stopwords = count_stopwords(text, "en");
    let result_stopwords = count_stopwords(&result, "en");

    assert!(
        result_stopwords < original_stopwords,
        "reduce_tokens should remove stopwords. Original: {}, Result: {}",
        original_stopwords,
        result_stopwords
    );

    assert!(
        result.contains("artificial") || result.contains("intelligence"),
        "Should preserve content words. Result: {}",
        result
    );
}

#[test]
fn test_stopwords_with_punctuation() {
    let config = TokenReductionConfig {
        level: ReductionLevel::Moderate,
        use_simd: false,
        ..Default::default()
    };

    let input = "The system, which is processing the data, uses the algorithm.";
    let result = reduce_tokens(input, &config, Some("en")).unwrap();

    assert!(
        !result.contains(" the ") || result.split_whitespace().filter(|w| w.contains("the")).count() < 3,
        "Should remove most instances of 'the'. Result: {}",
        result
    );

    assert!(
        result.contains("system") || result.contains("processing") || result.contains("algorithm"),
        "Should preserve content words. Result: {}",
        result
    );
}

#[test]
fn test_stopwords_with_numbers() {
    let config = TokenReductionConfig {
        level: ReductionLevel::Moderate,
        use_simd: false,
        ..Default::default()
    };

    let input = "The model has 100 layers and processes the data in 10 seconds";
    let result = reduce_tokens(input, &config, Some("en")).unwrap();

    assert!(
        result.contains("100"),
        "Should preserve number '100'. Result: {}",
        result
    );
    assert!(result.contains("10"), "Should preserve number '10'. Result: {}", result);

    assert!(
        result.contains("model") || result.contains("layers") || result.contains("processes"),
        "Should preserve content words. Result: {}",
        result
    );
}

#[test]
fn test_stopwords_removal_consistency_across_calls() {
    let config = TokenReductionConfig {
        level: ReductionLevel::Moderate,
        use_simd: false,
        ..Default::default()
    };

    let input = "The machine learning model is trained on the dataset";

    let result1 = reduce_tokens(input, &config, Some("en")).unwrap();
    let result2 = reduce_tokens(input, &config, Some("en")).unwrap();
    let result3 = reduce_tokens(input, &config, Some("en")).unwrap();

    assert_eq!(result1, result2, "Results should be consistent across calls");
    assert_eq!(result2, result3, "Results should be consistent across calls");
}

#[test]
fn test_stopwords_with_long_text() {
    let config = TokenReductionConfig {
        level: ReductionLevel::Moderate,
        use_simd: false,
        enable_parallel: false,
        ..Default::default()
    };

    let paragraph = "The machine learning model is trained on the large dataset. \
                     The training process uses the neural network architecture. \
                     The system processes the data efficiently and achieves the best performance. ";
    let input = paragraph.repeat(10);

    let result = reduce_tokens(&input, &config, Some("en")).unwrap();

    assert!(
        result.len() < input.len(),
        "Long stopword-heavy text should be reduced. Input: {} chars, Result: {} chars",
        input.len(),
        result.len()
    );

    let original_stopwords = count_stopwords(&input, "en");
    let result_stopwords = count_stopwords(&result, "en");

    assert!(
        result_stopwords < original_stopwords,
        "Should remove stopwords from long text. Original: {}, Result: {}",
        original_stopwords,
        result_stopwords
    );
}

#[test]
fn test_get_stopwords_with_fallback_in_reduction() {
    let primary_stopwords = get_stopwords_with_fallback("xyz", "en");
    assert!(primary_stopwords.is_some(), "Should fallback to English");

    let en_stopwords = get_stopwords("en").unwrap();
    assert_eq!(
        primary_stopwords.unwrap().len(),
        en_stopwords.len(),
        "Fallback should return English stopwords"
    );

    let config = TokenReductionConfig {
        level: ReductionLevel::Moderate,
        use_simd: false,
        ..Default::default()
    };

    let input = "The system is processing the data";
    let result = reduce_tokens(input, &config, Some("xyz")).unwrap();

    assert!(
        !result.contains(" the ") && !result.contains(" is "),
        "Should use fallback stopwords. Result: {}",
        result
    );
}

#[test]
fn test_stopwords_registry_completeness() {
    assert_eq!(STOPWORDS.len(), 64, "Should have exactly 64 language stopword sets");

    let en_stopwords = get_stopwords("en").expect("English stopwords must exist");
    assert!(en_stopwords.len() >= 70, "English should have at least 70 stopwords");

    assert!(en_stopwords.contains("the"), "Should contain 'the'");
    assert!(en_stopwords.contains("is"), "Should contain 'is'");
    assert!(en_stopwords.contains("and"), "Should contain 'and'");
    assert!(en_stopwords.contains("a"), "Should contain 'a'");
    assert!(en_stopwords.contains("an"), "Should contain 'an'");
    assert!(en_stopwords.contains("of"), "Should contain 'of'");
    assert!(en_stopwords.contains("to"), "Should contain 'to'");
    assert!(en_stopwords.contains("in"), "Should contain 'in'");
    assert!(en_stopwords.contains("for"), "Should contain 'for'");
}

#[test]
fn test_token_reduction_handles_nan_threshold() {
    let mut config = TokenReductionConfig {
        level: ReductionLevel::Maximum,
        semantic_threshold: f32::NAN,
        enable_semantic_clustering: true,
        target_reduction: Some(0.5),
        ..Default::default()
    };

    config.language_hint = Some("en".to_string());
    let input = "Critical system update highlights performance improvements across distributed modules.";

    let result = reduce_tokens(input, &config, Some("en")).unwrap_or_else(|_| String::new());
    assert!(
        result.chars().all(|c| !c.is_control()),
        "Result should not contain unexpected control characters"
    );
}

#[test]
fn test_token_reduction_handles_multibyte_utf8() {
    let config = TokenReductionConfig {
        level: ReductionLevel::Moderate,
        language_hint: Some("ja".to_string()),
        ..Default::default()
    };

    let input = "品質管理は重要です。🚀 高速抽出と漢字処理が求められています。";
    let result = reduce_tokens(input, &config, Some("ja")).unwrap();

    assert!(
        result.contains("品質管理") || result.contains("漢字処理"),
        "Important multibyte terms should survive reduction: {}",
        result
    );
}

#[test]
fn test_token_reduction_concurrent_access() {
    use std::sync::Arc;

    let config = Arc::new(TokenReductionConfig {
        level: ReductionLevel::Aggressive,
        enable_parallel: true,
        ..Default::default()
    });

    let input = "Concurrent reduction ensures thread safety without deadlocks or panics.";

    std::thread::scope(|scope| {
        for _ in 0..8 {
            let cfg = Arc::clone(&config);
            scope.spawn(move || {
                let reduced = reduce_tokens(input, &cfg, Some("en")).unwrap();
                assert!(!reduced.is_empty());
            });
        }
    });
}
#[test]
fn demo_stopwords_effectiveness() {
    use kreuzberg::stopwords::get_stopwords;
    use kreuzberg::text::token_reduction::{ReductionLevel, TokenReductionConfig, reduce_tokens};

    let en_text = "The machine learning model is trained on the large dataset and achieves good performance";
    let en_config = TokenReductionConfig {
        level: ReductionLevel::Moderate,
        use_simd: false,
        ..Default::default()
    };
    let en_result = reduce_tokens(en_text, &en_config, Some("en")).unwrap();

    println!("\n=== English Example ===");
    println!("BEFORE: {} chars", en_text.len());
    println!("{}", en_text);
    println!(
        "\nAFTER:  {} chars ({}% reduction)",
        en_result.len(),
        100 - (en_result.len() * 100 / en_text.len())
    );
    println!("{}", en_result);

    let zh_text = "这个人工智能系统可以处理自然语言";
    let zh_config = TokenReductionConfig {
        level: ReductionLevel::Moderate,
        use_simd: false,
        ..Default::default()
    };
    let zh_result = reduce_tokens(zh_text, &zh_config, Some("zh")).unwrap();

    println!("\n=== Chinese Example ===");
    println!("BEFORE: {}", zh_text);
    println!("AFTER:  {}", zh_result);

    let text = "The artificial intelligence system processes the natural language efficiently";

    println!("\n=== Reduction Level Comparison ===");
    println!("ORIGINAL: {}", text);

    for level in [
        ReductionLevel::Light,
        ReductionLevel::Moderate,
        ReductionLevel::Aggressive,
    ] {
        let config = TokenReductionConfig {
            level,
            use_simd: false,
            ..Default::default()
        };
        let result = reduce_tokens(text, &config, Some("en")).unwrap();
        println!(
            "{:?}: {} chars -> {} chars ({}% reduction)",
            level,
            text.len(),
            result.len(),
            100 - (result.len() * 100 / text.len())
        );
        println!("  {}", result);
    }

    let stopwords = get_stopwords("en").unwrap();
    println!("\n=== Stopwords Stats ===");
    println!("English stopwords: {}", stopwords.len());
    println!("Sample stopwords: {:?}", stopwords.iter().take(10).collect::<Vec<_>>());
}
