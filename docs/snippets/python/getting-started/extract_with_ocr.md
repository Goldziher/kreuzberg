```python
import asyncio
from kreuzberg import extract_file, ExtractionConfig, OcrConfig, TesseractConfig

async def main() -> None:
    config = ExtractionConfig(
        force_ocr=True,
        ocr=OcrConfig(
            backend="tesseract",
            language="eng",
            tesseract_config=TesseractConfig(psm=3)
        )
    )
    result = await extract_file("scanned.pdf", config=config)
    print(result.content)
    print(f"Detected Languages: {result.detected_languages}")

asyncio.run(main())
```
