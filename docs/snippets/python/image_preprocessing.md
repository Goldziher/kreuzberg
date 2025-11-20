```python
from kreuzberg import ExtractionConfig, OcrConfig, TesseractConfig, ImagePreprocessingConfig

config = ExtractionConfig(
    ocr=OcrConfig(
        tesseract_config=TesseractConfig(
            preprocessing=ImagePreprocessingConfig(
                target_dpi=300,
                denoise=True,
                deskew=True,
                contrast_enhance=True,
                binarization_method="otsu"
            )
        )
    )
)
```
