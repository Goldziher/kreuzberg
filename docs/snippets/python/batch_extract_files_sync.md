```python
from kreuzberg import batch_extract_files_sync, ExtractionConfig

files = ["doc1.pdf", "doc2.docx", "doc3.pptx"]
config = ExtractionConfig()

results = batch_extract_files_sync(files, config=config)

for i, result in enumerate(results):
    print(f"File {i+1}: {len(result.content)} characters")
```
