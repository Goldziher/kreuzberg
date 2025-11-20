```python
from kreuzberg import extract_file_sync, ExtractionConfig, OcrConfig, PdfConfig

config = ExtractionConfig(
    ocr=OcrConfig(backend="tesseract"),
    pdf=PdfConfig(dpi=300)
)

result = extract_file_sync("scanned.pdf", config=config)
```
