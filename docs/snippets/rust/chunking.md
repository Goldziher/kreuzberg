```rust
use kreuzberg::{ChunkingConfig, EmbeddingConfig, EmbeddingModelType, ExtractionConfig};

fn main() {
    let config = ExtractionConfig {
        chunking: Some(ChunkingConfig {
            max_chars: Some(1500),
            max_overlap: Some(200),
            embedding: Some(EmbeddingConfig {
                model: Some(EmbeddingModelType {
                    r#type: "preset".to_string(),
                    name: Some("text-embedding-all-minilm-l6-v2".to_string()),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    };
    println!("{:?}", config.chunking);
}
```
