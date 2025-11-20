```python
from kreuzberg import (
    extract_file_sync,
    ExtractionConfig,
    OcrConfig,
    ChunkingConfig,
)

config = ExtractionConfig(
    use_cache=True,
    ocr=OcrConfig(
        backend="tesseract",
        language="eng",
    ),
    chunking=ChunkingConfig(
        max_chunk_size=1000,
        overlap=200,
    ),
)

result = extract_file_sync("document.pdf", config)
print(f"Content length: {len(result.content)}")
```
