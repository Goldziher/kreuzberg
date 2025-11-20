```python
from kreuzberg import (
    extract_file_sync,
    ExtractionConfig,
    OcrConfig,
    ChunkingConfig,
    TokenReductionConfig,
    LanguageDetectionConfig
)

config = ExtractionConfig(
    # Enable OCR
    ocr=OcrConfig(
        backend="tesseract",
        language="eng+deu"  # Multiple languages
    ),

    # Enable chunking for LLM processing
    chunking=ChunkingConfig(
        max_chunk_size=1000,
        overlap=100
    ),

    # Enable token reduction
    token_reduction=TokenReductionConfig(
        enabled=True,
        target_reduction=0.3  # Reduce by 30%
    ),

    # Enable language detection
    language_detection=LanguageDetectionConfig(
        enabled=True,
        detect_multiple=True
    ),

    # Enable caching
    use_cache=True,

    # Enable quality processing
    enable_quality_processing=True
)

result = extract_file_sync("document.pdf", config=config)

# Access chunks
for chunk in result.chunks:
    print(f"Chunk: {chunk.text[:100]}...")

# Access detected languages
if result.detected_languages:
    print(f"Languages: {result.detected_languages}")
```
