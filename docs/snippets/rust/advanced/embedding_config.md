```rust title="Rust"
use kreuzberg::{ChunkingConfig, EmbeddingConfig, EmbeddingModelType, ExtractionConfig};

fn main() {
    let config = ExtractionConfig {
        chunking: Some(ChunkingConfig {
            max_chars: Some(1000),
            embedding: Some(EmbeddingConfig {
                model: Some(EmbeddingModelType {
                    r#type: "preset".to_string(),
                    name: Some("all-mpnet-base-v2".to_string()),
                    ..Default::default()
                }),
                batch_size: Some(16),
                normalize: Some(true),
                show_download_progress: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    };
    println!("{:?}", config.chunking);
}
```
