```python
from kreuzberg import batch_extract_bytes_sync, ExtractionConfig

files = ["doc1.pdf", "doc2.docx"]
data_list = []
mime_types = []

for file in files:
    with open(file, "rb") as f:
        data_list.append(f.read())
    mime_types.append(
        "application/pdf" if file.endswith(".pdf") else
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
    )

results = batch_extract_bytes_sync(
    data_list,
    mime_types=mime_types,
    config=ExtractionConfig()
)

for i, result in enumerate(results):
    print(f"Document {i+1}: {len(result.content)} characters")
```
