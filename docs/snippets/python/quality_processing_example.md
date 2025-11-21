```python
from kreuzberg import extract_file, ExtractionConfig

config = ExtractionConfig(enable_quality_processing=True)
result = extract_file("scanned_document.pdf", config=config)

quality_score = result.metadata.get("quality_score", 0.0)

if quality_score < 0.5:
    print(f"Warning: Low quality extraction ({quality_score:.2f})")
    print("Consider re-scanning with higher DPI or adjusting OCR settings")
else:
    print(f"Quality score: {quality_score:.2f}")
```
