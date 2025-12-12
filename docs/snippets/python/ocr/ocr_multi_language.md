```python title="Python"
from kreuzberg import extract_file_sync, ExtractionConfig, OcrConfig

config: ExtractionConfig = ExtractionConfig(
    ocr=OcrConfig(backend="tesseract", language="eng+deu+fra")
)

result = extract_file_sync("multilingual.pdf", config=config)

content: str = result.content
preview: str = content[:100]
total_length: int = len(content)

print(f"Extracted content (preview): {preview}")
print(f"Total characters: {total_length}")
```
