```python
from kreuzberg import ExtractionConfig, extract_file

config = ExtractionConfig.discover()
result = extract_file("document.pdf", config=config)
```
