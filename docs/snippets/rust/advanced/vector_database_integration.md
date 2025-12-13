```rust title="Rust"
use kreuzberg::{extract_file, ExtractionConfig, ChunkingConfig, EmbeddingConfig};

struct VectorRecord {
    id: String,
    content: String,
    embedding: Vec<f32>,
    metadata: std::collections::HashMap<String, String>,
}

async fn extract_and_vectorize(
    document_path: &str,
    document_id: &str,
) -> Result<Vec<VectorRecord>, Box<dyn std::error::Error>> {
    let config = ExtractionConfig {
        chunking: Some(ChunkingConfig {
            max_chars: 512,
            max_overlap: 50,
            embedding: Some(EmbeddingConfig {
                model: kreuzberg::EmbeddingModelType::Preset {
                    name: "balanced".to_string(),
                },
                normalize: true,
                batch_size: 32,
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    };

    let result = extract_file(document_path, None, &config).await?;

    let mut records = Vec::new();
    if let Some(chunks) = result.chunks {
        for (index, chunk) in chunks.iter().enumerate() {
            if let Some(embedding) = &chunk.embedding {
                let mut metadata = std::collections::HashMap::new();
                metadata.insert("document_id".to_string(), document_id.to_string());
                metadata.insert("chunk_index".to_string(), index.to_string());
                metadata.insert("content_length".to_string(), chunk.content.len().to_string());

                records.push(VectorRecord {
                    id: format!("{}_chunk_{}", document_id, index),
                    content: chunk.content.clone(),
                    embedding: embedding.clone(),
                    metadata,
                });
            }
        }
    }

    Ok(records)
}
```
