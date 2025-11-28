```python
from kreuzberg import (
    extract_file_sync,
    ExtractionConfig,
    OcrConfig,
    ChunkingConfig,
)

config: ExtractionConfig = ExtractionConfig(
    use_cache=True,
    ocr=OcrConfig(backend="tesseract", language="eng"),
    chunking=ChunkingConfig(max_chars=1000, max_overlap=200),
)

result = extract_file_sync("document.pdf", config=config)
content_length: int = len(result.content)
print(f"Content length: {content_length}")
```
