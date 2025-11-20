```python
from kreuzberg import extract_file_sync, ExtractionConfig

result = extract_file_sync("document.pdf", config=ExtractionConfig())

# Iterate over tables
for table in result.tables:
    print(f"Table with {len(table.cells)} rows")
    print(table.markdown)  # Markdown representation

    # Access cells
    for row in table.cells:
        print(row)
```
