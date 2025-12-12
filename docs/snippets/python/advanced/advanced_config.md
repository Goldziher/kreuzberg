```python title="Python"
from kreuzberg import (
    extract_file_sync,
    ExtractionConfig,
    OcrConfig,
    ChunkingConfig,
    TokenReductionConfig,
    LanguageDetectionConfig,
)

config = ExtractionConfig(
    ocr=OcrConfig(backend="tesseract", language="eng+deu"),
    chunking=ChunkingConfig(max_chars=1000, max_overlap=100),
    token_reduction=TokenReductionConfig(enabled=True),
    language_detection=LanguageDetectionConfig(
        enabled=True, detect_multiple=True
    ),
    use_cache=True,
    enable_quality_processing=True,
)

result = extract_file_sync("document.pdf", config=config)

for chunk in result.chunks:
    print(f"Chunk: {chunk.content[:100]}")

if result.detected_languages:
    print(f"Languages: {result.detected_languages}")
```
