```python
from kreuzberg import ExtractionConfig, OcrConfig, TesseractConfig

config = ExtractionConfig(
    ocr=OcrConfig(
        backend="tesseract",
        language="eng+fra",
        tesseract_config=TesseractConfig(psm=3)
    )
)
```
