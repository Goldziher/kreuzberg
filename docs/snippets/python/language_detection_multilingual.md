```python
from kreuzberg import extract_file, ExtractionConfig, LanguageDetectionConfig

config = ExtractionConfig(
    language_detection=LanguageDetectionConfig(
        enabled=True,
        min_confidence=0.8,
        detect_multiple=True
    )
)

result = extract_file("multilingual_document.pdf", config=config)

print(f"Detected languages: {result.detected_languages}")
# Output: ['eng', 'fra', 'deu']
```
