```python title="Python"
import asyncio
from kreuzberg import (
    ExtractionConfig,
    OcrConfig,
    TesseractConfig,
    ImagePreprocessingConfig,
    extract_file,
)

async def main() -> None:
    config: ExtractionConfig = ExtractionConfig(
        ocr=OcrConfig(
            tesseract_config=TesseractConfig(
                preprocessing=ImagePreprocessingConfig(
                    target_dpi=300,
                    denoise=True,
                    deskew=True,
                    contrast_enhance=True,
                    binarization_method="otsu",
                )
            )
        )
    )
    result = await extract_file("scanned.pdf", config=config)
    print(f"Content: {result.content[:100]}")

asyncio.run(main())
```
