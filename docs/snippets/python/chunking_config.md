```python
from kreuzberg import ExtractionConfig, ChunkingConfig

config = ExtractionConfig(
    chunking=ChunkingConfig(
        max_chars=1000,        # Maximum chunk size
        max_overlap=200        # Overlap between chunks
    )
)
```
