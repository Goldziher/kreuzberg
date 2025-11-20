```python
from kreuzberg import ExtractionConfig, ChunkingConfig, EmbeddingConfig, EmbeddingModelType

config = ExtractionConfig(
    chunking=ChunkingConfig(
        max_chars=1500,
        max_overlap=200,
        embedding=EmbeddingConfig(
            model=EmbeddingModelType(
                type="preset",
                name="text-embedding-all-minilm-l6-v2"
            )
        )
    )
)
```
