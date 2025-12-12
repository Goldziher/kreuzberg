```rust title="Rust"
use kreuzberg::{ExtractionConfig, KeywordConfig, KeywordAlgorithm};

let config = ExtractionConfig {
    keywords: Some(KeywordConfig {
        algorithm: KeywordAlgorithm::Yake,
        max_keywords: 10,
        min_score: 0.3,
        ngram_range: (1, 3),
        language: Some("en".to_string()),
        ..Default::default()
    }),
    ..Default::default()
};
```
