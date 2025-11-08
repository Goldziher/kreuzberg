//! Embedding generation support for RAG (Retrieval-Augmented Generation) systems.
//!
//! This module provides text embedding generation using ONNX models via fastembed-rs.
//! Embeddings can be generated for text chunks to enable semantic search and RAG pipelines.
//!
//! # Features
//!
//! - Multiple pre-configured models optimized for different use cases
//! - Preset configurations for common RAG scenarios
//! - Full customization of model location and parameters
//! - Batch processing for efficient embedding generation
//! - Optional GPU acceleration via ONNX Runtime execution providers
//!
//! # Example
//!
//! ```rust,ignore
//! use kreuzberg::{extract_file, ExtractionConfig, ChunkingConfig, EmbeddingConfig};
//!
//! let config = ExtractionConfig {
//!     chunking: Some(ChunkingConfig {
//!         preset: Some("balanced".to_string()),
//!         embedding: Some(EmbeddingConfig::default()),
//!         ..Default::default()
//!     }),
//!     ..Default::default()
//! };
//!
//! let result = extract_file("document.pdf", None, &config).await?;
//! for chunk in result.chunks.unwrap() {
//!     if let Some(embedding) = chunk.embedding {
//!         println!("Chunk has {} dimension embedding", embedding.len());
//!     }
//! }
//! ```

#[cfg(feature = "embeddings")]
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};

#[cfg(feature = "embeddings")]
use std::sync::{Arc, Mutex, RwLock};

#[cfg(feature = "embeddings")]
use std::collections::HashMap;

#[cfg(feature = "embeddings")]
use lazy_static::lazy_static;

#[cfg(feature = "embeddings")]
lazy_static! {
    static ref MODEL_CACHE: RwLock<HashMap<String, Arc<Mutex<TextEmbedding>>>> = RwLock::new(HashMap::new());
}

/// Get or initialize a text embedding model from cache.
///
/// This function ensures models are initialized only once and reused across
/// the application, avoiding redundant downloads and initialization overhead.
#[cfg(feature = "embeddings")]
pub fn get_or_init_model(
    model: EmbeddingModel,
    cache_dir: Option<std::path::PathBuf>,
) -> crate::Result<Arc<Mutex<TextEmbedding>>> {
    let cache_directory = cache_dir.unwrap_or_else(|| {
        let mut path = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        path.push(".kreuzberg");
        path.push("embeddings");
        path
    });

    let model_key = format!("{:?}_{}", model, cache_directory.display());

    {
        let cache = MODEL_CACHE.read().map_err(|e| crate::KreuzbergError::Plugin {
            message: format!("Failed to acquire model cache read lock: {}", e),
            plugin_name: "embeddings".to_string(),
        })?;

        if let Some(cached_model) = cache.get(&model_key) {
            return Ok(Arc::clone(cached_model));
        }
    }

    {
        let mut cache = MODEL_CACHE.write().map_err(|e| crate::KreuzbergError::Plugin {
            message: format!("Failed to acquire model cache write lock: {}", e),
            plugin_name: "embeddings".to_string(),
        })?;

        if let Some(cached_model) = cache.get(&model_key) {
            return Ok(Arc::clone(cached_model));
        }

        let mut init_options = InitOptions::new(model);
        init_options = init_options.with_cache_dir(cache_directory);

        let embedding_model = TextEmbedding::try_new(init_options).map_err(|e| crate::KreuzbergError::Plugin {
            message: format!("Failed to initialize embedding model: {}", e),
            plugin_name: "embeddings".to_string(),
        })?;

        let arc_model = Arc::new(Mutex::new(embedding_model));
        cache.insert(model_key, Arc::clone(&arc_model));

        Ok(arc_model)
    }
}

/// Preset configurations for common RAG use cases.
///
/// Each preset combines chunk size, overlap, and embedding model
/// to provide an optimized configuration for specific scenarios.
#[derive(Debug, Clone)]
pub struct EmbeddingPreset {
    pub name: &'static str,
    pub chunk_size: usize,
    pub overlap: usize,
    #[cfg(feature = "embeddings")]
    pub model: EmbeddingModel,
    #[cfg(not(feature = "embeddings"))]
    pub model_name: &'static str,
    pub dimensions: usize,
    pub description: &'static str,
}

/// All available embedding presets.
pub const EMBEDDING_PRESETS: &[EmbeddingPreset] = &[
    EmbeddingPreset {
        name: "fast",
        chunk_size: 512,
        overlap: 50,
        #[cfg(feature = "embeddings")]
        model: EmbeddingModel::AllMiniLML6V2Q,
        #[cfg(not(feature = "embeddings"))]
        model_name: "AllMiniLML6V2Q",
        dimensions: 384,
        description: "Fast embedding with quantized model (384 dims, ~22M params). Best for: Quick prototyping, development, resource-constrained environments.",
    },
    EmbeddingPreset {
        name: "balanced",
        chunk_size: 1024,
        overlap: 100,
        #[cfg(feature = "embeddings")]
        model: EmbeddingModel::BGEBaseENV15,
        #[cfg(not(feature = "embeddings"))]
        model_name: "BGEBaseENV15",
        dimensions: 768,
        description: "Balanced quality and speed (768 dims, ~109M params). Best for: General-purpose RAG, production deployments, English documents.",
    },
    EmbeddingPreset {
        name: "quality",
        chunk_size: 2000,
        overlap: 200,
        #[cfg(feature = "embeddings")]
        model: EmbeddingModel::BGELargeENV15,
        #[cfg(not(feature = "embeddings"))]
        model_name: "BGELargeENV15",
        dimensions: 1024,
        description: "High quality with larger context (1024 dims, ~335M params). Best for: Complex documents, maximum accuracy, sufficient compute resources.",
    },
    EmbeddingPreset {
        name: "multilingual",
        chunk_size: 1024,
        overlap: 100,
        #[cfg(feature = "embeddings")]
        model: EmbeddingModel::MultilingualE5Base,
        #[cfg(not(feature = "embeddings"))]
        model_name: "MultilingualE5Base",
        dimensions: 768,
        description: "Multilingual support (768 dims, 100+ languages). Best for: International documents, mixed-language content, global applications.",
    },
];

/// Get a preset by name.
pub fn get_preset(name: &str) -> Option<&'static EmbeddingPreset> {
    EMBEDDING_PRESETS.iter().find(|p| p.name == name)
}

/// List all available preset names.
pub fn list_presets() -> Vec<&'static str> {
    EMBEDDING_PRESETS.iter().map(|p| p.name).collect()
}

/// Generate embeddings for text chunks using the specified configuration.
///
/// This function modifies chunks in-place, populating their `embedding` field
/// with generated embedding vectors. It uses batch processing for efficiency.
///
/// # Arguments
///
/// * `chunks` - Mutable reference to vector of chunks to generate embeddings for
/// * `config` - Embedding configuration specifying model and parameters
///
/// # Returns
///
/// Returns `Ok(())` if embeddings were generated successfully, or an error if
/// model initialization or embedding generation fails.
///
/// # Example
///
/// ```rust,ignore
/// let mut chunks = vec![
///     Chunk { content: "Hello world".to_string(), embedding: None, metadata: ... },
///     Chunk { content: "Second chunk".to_string(), embedding: None, metadata: ... },
/// ];
/// let config = EmbeddingConfig::default();
/// generate_embeddings_for_chunks(&mut chunks, &config)?;
/// // Now chunks have embeddings populated
/// ```
#[cfg(feature = "embeddings")]
pub fn generate_embeddings_for_chunks(
    chunks: &mut [crate::types::Chunk],
    config: &crate::core::config::EmbeddingConfig,
) -> crate::Result<()> {
    if chunks.is_empty() {
        return Ok(());
    }

    let fastembed_model = match &config.model {
        crate::core::config::EmbeddingModelType::Preset { name } => {
            let preset = get_preset(name).ok_or_else(|| crate::KreuzbergError::Plugin {
                message: format!("Unknown embedding preset: {}", name),
                plugin_name: "embeddings".to_string(),
            })?;
            preset.model.clone()
        }
        #[cfg(feature = "embeddings")]
        crate::core::config::EmbeddingModelType::FastEmbed { model, .. } => match model.as_str() {
            "AllMiniLML6V2Q" => fastembed::EmbeddingModel::AllMiniLML6V2Q,
            "BGEBaseENV15" => fastembed::EmbeddingModel::BGEBaseENV15,
            "BGELargeENV15" => fastembed::EmbeddingModel::BGELargeENV15,
            "MultilingualE5Base" => fastembed::EmbeddingModel::MultilingualE5Base,
            _ => {
                return Err(crate::KreuzbergError::Plugin {
                    message: format!("Unknown fastembed model: {}", model),
                    plugin_name: "embeddings".to_string(),
                });
            }
        },
        crate::core::config::EmbeddingModelType::Custom { .. } => {
            return Err(crate::KreuzbergError::Plugin {
                message: "Custom ONNX models are not yet supported for embedding generation".to_string(),
                plugin_name: "embeddings".to_string(),
            });
        }
    };

    let model = get_or_init_model(fastembed_model, config.cache_dir.clone())?;

    let texts: Vec<String> = chunks.iter().map(|chunk| chunk.content.clone()).collect();

    let embeddings_result = {
        let mut locked_model = model.lock().map_err(|e| crate::KreuzbergError::Plugin {
            message: format!("Failed to acquire model lock: {}", e),
            plugin_name: "embeddings".to_string(),
        })?;

        locked_model
            .embed(texts, Some(config.batch_size))
            .map_err(|e| crate::KreuzbergError::Plugin {
                message: format!("Failed to generate embeddings: {}", e),
                plugin_name: "embeddings".to_string(),
            })?
    };

    for (chunk, mut embedding) in chunks.iter_mut().zip(embeddings_result.into_iter()) {
        if config.normalize {
            let magnitude: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
            if magnitude > 0.0 {
                embedding.iter_mut().for_each(|x| *x /= magnitude);
            }
        }

        chunk.embedding = Some(embedding);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_preset() {
        assert!(get_preset("balanced").is_some());
        assert!(get_preset("fast").is_some());
        assert!(get_preset("quality").is_some());
        assert!(get_preset("multilingual").is_some());
        assert!(get_preset("nonexistent").is_none());
    }

    #[test]
    fn test_list_presets() {
        let presets = list_presets();
        assert_eq!(presets.len(), 4);
        assert!(presets.contains(&"fast"));
        assert!(presets.contains(&"balanced"));
        assert!(presets.contains(&"quality"));
        assert!(presets.contains(&"multilingual"));
    }

    #[test]
    fn test_preset_dimensions() {
        let balanced = get_preset("balanced").unwrap();
        assert_eq!(balanced.dimensions, 768);

        let fast = get_preset("fast").unwrap();
        assert_eq!(fast.dimensions, 384);

        let quality = get_preset("quality").unwrap();
        assert_eq!(quality.dimensions, 1024);
    }

    #[test]
    fn test_preset_chunk_sizes() {
        let fast = get_preset("fast").unwrap();
        assert_eq!(fast.chunk_size, 512);
        assert_eq!(fast.overlap, 50);

        let quality = get_preset("quality").unwrap();
        assert_eq!(quality.chunk_size, 2000);
        assert_eq!(quality.overlap, 200);
    }
}
