```python title="Python"
from kreuzberg import (
    ExtractionConfig,
    ChunkingConfig,
    EmbeddingConfig,
    EmbeddingModelType,
)

config: ExtractionConfig = ExtractionConfig(
    chunking=ChunkingConfig(
        max_chars=1024,
        max_overlap=100,
        embedding=EmbeddingConfig(
            model=EmbeddingModelType.preset("balanced"),
            normalize=True,
            batch_size=32,
            show_download_progress=False,
        ),
    )
)
```
