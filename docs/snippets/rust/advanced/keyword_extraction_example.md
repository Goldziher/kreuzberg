```rust title="Rust"
use kreuzberg::{extract_file, ExtractionConfig, KeywordConfig, KeywordAlgorithm};

let config = ExtractionConfig {
    keywords: Some(KeywordConfig {
        algorithm: KeywordAlgorithm::Yake,
        max_keywords: 10,
        min_score: 0.3,
        ..Default::default()
    }),
    ..Default::default()
};

let result = extract_file("research_paper.pdf", None, &config).await?;

if let Some(keywords) = result.metadata.additional.get("keywords") {
    println!("Keywords: {:?}", keywords);
}
```
