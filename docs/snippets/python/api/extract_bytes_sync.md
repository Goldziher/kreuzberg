```python title="Python"
from kreuzberg import extract_bytes_sync, ExtractionConfig

with open("document.pdf", "rb") as f:
    data = f.read()

result = extract_bytes_sync(
    data,
    mime_type="application/pdf",
    config=ExtractionConfig()
)
print(result.content)
```
