```python title="Python"
from kreuzberg import batch_extract_files_sync, ExtractionConfig

files: list[str] = ["doc1.pdf", "doc2.docx", "doc3.pptx"]
config: ExtractionConfig = ExtractionConfig()

results = batch_extract_files_sync(files, config=config)

for i, result in enumerate(results):
    char_count: int = len(result.content)
    print(f"File {i + 1}: {char_count} characters")
```
