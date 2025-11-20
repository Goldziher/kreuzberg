```python
from kreuzberg import extract_file_sync, ExtractionConfig, OcrConfig

config = ExtractionConfig(
    ocr=OcrConfig(
        backend="tesseract",
        language="eng+deu+fra"
    )
)

result = extract_file_sync("multilingual.pdf", config=config)
print(result.content)
```
