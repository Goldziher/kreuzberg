```python
from kreuzberg import extract_file_sync, ExtractionConfig

config: ExtractionConfig = ExtractionConfig()
result = extract_file_sync("document.pdf", config=config)

content: str = result.content
table_count: int = len(result.tables)
metadata: dict = result.metadata

print(f"Content length: {len(content)} characters")
print(f"Tables: {table_count}")
print(f"Metadata keys: {list(metadata.keys())}")
```
