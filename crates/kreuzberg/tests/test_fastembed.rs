//! Integration tests for fastembed embeddings

#[cfg(feature = "embeddings")]
#[tokio::test]
async fn test_fastembed_initialization() {
    use fastembed::TextEmbedding;

    // Test initializing the smallest/fastest model
    let model = TextEmbedding::try_new(Default::default());

    assert!(model.is_ok(), "Failed to initialize fastembed model: {:?}", model.err());
}

#[cfg(feature = "embeddings")]
#[tokio::test]
async fn test_fastembed_embedding_generation() {
    use fastembed::TextEmbedding;

    // Initialize model
    let mut model = TextEmbedding::try_new(Default::default()).expect("Failed to initialize model");

    // Generate embeddings for sample texts
    let texts = vec![
        "Hello world, this is a test.",
        "Fastembed is a Rust embedding library.",
        "Testing embedding generation stability.",
    ];

    let result = model.embed(texts.clone(), None);
    assert!(result.is_ok(), "Failed to generate embeddings: {:?}", result.err());

    let embeddings = result.unwrap();
    assert_eq!(embeddings.len(), 3, "Expected 3 embeddings");

    // Verify embedding dimensions (AllMiniLML6V2Q produces 384-dim embeddings)
    for (i, embedding) in embeddings.iter().enumerate() {
        assert_eq!(embedding.len(), 384, "Embedding {} has wrong dimensions", i);

        // Verify embeddings are not all zeros
        let sum: f32 = embedding.iter().sum();
        assert!(sum.abs() > 0.0001, "Embedding {} appears to be all zeros", i);
    }

    println!(
        "✓ Successfully generated {} embeddings with 384 dimensions each",
        embeddings.len()
    );
}

#[cfg(feature = "embeddings")]
#[tokio::test]
async fn test_fastembed_batch_processing() {
    use fastembed::TextEmbedding;

    let mut model = TextEmbedding::try_new(Default::default()).expect("Failed to initialize model");

    // Test with a larger batch
    let texts: Vec<String> = (0..50)
        .map(|i| {
            format!(
                "This is test sentence number {}. It contains some text for embedding.",
                i
            )
        })
        .collect();

    let start = std::time::Instant::now();
    let result = model.embed(texts.clone(), Some(32)); // batch_size=32
    let duration = start.elapsed();

    assert!(result.is_ok(), "Batch embedding failed: {:?}", result.err());

    let embeddings = result.unwrap();
    assert_eq!(embeddings.len(), 50, "Expected 50 embeddings");

    println!(
        "✓ Batch processed 50 texts in {:?} ({:.2} ms per text)",
        duration,
        duration.as_millis() as f64 / 50.0
    );
}

#[cfg(feature = "embeddings")]
#[tokio::test]
async fn test_fastembed_different_models() {
    use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};

    let models_to_test = vec![
        (EmbeddingModel::AllMiniLML6V2Q, 384, "AllMiniLML6V2Q (fast, quantized)"),
        (EmbeddingModel::BGEBaseENV15, 768, "BGEBaseENV15 (balanced)"),
    ];

    let test_text = vec!["Hello world"];

    for (model_type, expected_dims, description) in models_to_test {
        println!("Testing {}", description);

        let model = TextEmbedding::try_new(InitOptions::new(model_type));

        match model {
            Ok(mut m) => {
                let result = m.embed(test_text.clone(), None);
                assert!(result.is_ok(), "Failed to generate embedding for {}", description);

                let embeddings = result.unwrap();
                assert_eq!(embeddings.len(), 1);
                assert_eq!(
                    embeddings[0].len(),
                    expected_dims,
                    "Wrong dimensions for {}",
                    description
                );

                println!("  ✓ {} produces {}-dim embeddings", description, expected_dims);
            }
            Err(e) => {
                println!("  ⚠ Failed to initialize {}: {:?}", description, e);
                // Don't fail the test - model download might fail in CI
            }
        }
    }
}

#[cfg(feature = "embeddings")]
#[tokio::test]
async fn test_fastembed_error_handling() {
    use fastembed::TextEmbedding;

    // Test empty input
    let mut model = TextEmbedding::try_new(Default::default()).expect("Failed to initialize model");

    let empty_texts: Vec<String> = vec![];
    let result = model.embed(empty_texts, None);

    // fastembed should handle empty input gracefully
    match result {
        Ok(embeddings) => assert_eq!(embeddings.len(), 0, "Empty input should produce empty output"),
        Err(_) => {
            // Also acceptable if it returns an error
            println!("  ℹ fastembed returns error for empty input (acceptable)");
        }
    }
}
