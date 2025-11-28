```python
from kreuzberg import extract_file_sync, ExtractionConfig, ExtractedTable

result = extract_file_sync("document.pdf", config=ExtractionConfig())

for table in result.tables:
    row_count: int = len(table.cells)
    print(f"Table with {row_count} rows")
    print(table.markdown)
    for row in table.cells:
        print(row)
```
