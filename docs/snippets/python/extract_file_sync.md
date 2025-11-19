```python
from kreuzberg import extract_file_sync, ExtractionConfig

result = extract_file_sync("document.pdf", config=ExtractionConfig())

print(result.content)
print(f"Tables: {len(result.tables)}")
print(f"Metadata: {result.metadata}")
```
