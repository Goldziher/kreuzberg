```python
from kreuzberg import ExtractionConfig, ChunkingConfig, EmbeddingConfig, EmbeddingModelType

config = ExtractionConfig(
    chunking=ChunkingConfig(
        max_chars=1024,
        max_overlap=100,
        embedding=EmbeddingConfig(
            model=EmbeddingModelType.preset("balanced"),  # EmbeddingModelType object
            normalize=True,             # L2 normalization for cosine similarity
            batch_size=32,              # Batch processing size
            show_download_progress=False
        )
    )
)
```
