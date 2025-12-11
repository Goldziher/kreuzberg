```python
from kreuzberg import register_document_extractor

class CustomExtractor:
    def name(self) -> str:
        return "custom"

    def version(self) -> str:
        return "1.0.0"

extractor = CustomExtractor()
register_document_extractor(extractor)
print("Extractor registered")
```
