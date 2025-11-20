```python
from kreuzberg import ExtractionConfig, OcrConfig, TesseractConfig

config = ExtractionConfig(
    ocr=OcrConfig(
        language="eng+fra+deu",
        tesseract_config=TesseractConfig(
            psm=6,
            oem=1,
            min_confidence=0.8,
            tessedit_char_whitelist="ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789 .,!?",
            enable_table_detection=True
        )
    )
)
```
