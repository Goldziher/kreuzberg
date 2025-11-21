```python
from kreuzberg import extract_file_sync, ExtractionConfig, OcrConfig

config = ExtractionConfig(
    ocr=OcrConfig(
        backend="easyocr",
        language="en",
        use_gpu=True
    )
)

result = extract_file_sync("scanned.pdf", config=config)
print(result.content)
```
