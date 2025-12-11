```python
from kreuzberg import batch_extract_bytes_sync, ExtractionConfig

files: list[str] = ["doc1.pdf", "doc2.docx"]
data_list: list[bytes] = []
mime_types: list[str] = []

for file in files:
    with open(file, "rb") as f:
        data_list.append(f.read())
    mime_type: str = "application/pdf" if file.endswith(".pdf") else "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
    mime_types.append(mime_type)

config: ExtractionConfig = ExtractionConfig()
results = batch_extract_bytes_sync(data_list, mime_types, config=config)

for i, result in enumerate(results):
    char_count: int = len(result.content)
    print(f"Document {i + 1}: {char_count} characters")
```
