```python
from kreuzberg import extract_file, ExtractionConfig

config = ExtractionConfig(
    use_cache=True,
    enable_quality_processing=True
)

result = extract_file("document.pdf", config=config)
```
