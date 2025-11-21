```python
from kreuzberg import (
    extract_file,
    ExtractionConfig,
    ChunkingConfig,
    EmbeddingConfig,
    EmbeddingModelType,
    LanguageDetectionConfig,
    TokenReductionConfig,
    KeywordConfig,
    KeywordAlgorithm
)

config = ExtractionConfig(
    # Enable quality scoring
    enable_quality_processing=True,

    # Detect languages
    language_detection=LanguageDetectionConfig(
        enabled=True,
        detect_multiple=True
    ),

    # Reduce tokens before chunking
    token_reduction=TokenReductionConfig(
        mode="moderate",
        preserve_markdown=True
    ),

    # Chunk with embeddings
    chunking=ChunkingConfig(
        max_chars=512,
        max_overlap=50,
        embedding=EmbeddingConfig(
            model=EmbeddingModelType.preset("balanced"),
            normalize=True
        )
    ),

    # Extract keywords
    keywords=KeywordConfig(
        algorithm=KeywordAlgorithm.YAKE,
        max_keywords=10
    )
)

result = extract_file("document.pdf", config=config)

print(f"Quality: {result.metadata['quality_score']:.2f}")
print(f"Languages: {result.detected_languages}")
print(f"Keywords: {[kw['text'] for kw in result.metadata['keywords']]}")
if result.chunks and result.chunks[0].embedding:
    print(f"Chunks: {len(result.chunks)} with {len(result.chunks[0].embedding)} dimensions")
```
