```python
from kreuzberg import ExtractionConfig, ChunkingConfig, EmbeddingConfig, EmbeddingModelType

config = ExtractionConfig(
    chunking=ChunkingConfig(
        max_chars=1000,
        embedding=EmbeddingConfig(
            model=EmbeddingModelType.preset("all-mpnet-base-v2"),
            batch_size=16,
            normalize=True,
            show_download_progress=True
        )
    )
)
```
