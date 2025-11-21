```python
from kreuzberg import ExtractionConfig, LanguageDetectionConfig

config = ExtractionConfig(
    language_detection=LanguageDetectionConfig(
        enabled=True,
        min_confidence=0.8,      # Confidence threshold (0.0-1.0)
        detect_multiple=False    # Single vs. multiple languages
    )
)
```
