```python
from kreuzberg import extract_file_sync, ExtractionConfig, OcrConfig

config = ExtractionConfig(
    ocr=OcrConfig(backend="tesseract"),
    force_ocr=True
)

result = extract_file_sync("document.pdf", config=config)
print(result.content)
```
