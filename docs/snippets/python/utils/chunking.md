```python
from kreuzberg import (
    ExtractionConfig,
    ChunkingConfig,
    EmbeddingConfig,
    EmbeddingModelType,
)

config: ExtractionConfig = ExtractionConfig(
    chunking=ChunkingConfig(
        max_chars=1500,
        max_overlap=200,
        embedding=EmbeddingConfig(
            model=EmbeddingModelType.preset("all-minilm-l6-v2")
        ),
    )
)
```
