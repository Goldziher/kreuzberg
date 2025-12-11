```python
import asyncio
from kreuzberg import ExtractionConfig, OcrConfig, TesseractConfig, extract_file

async def main() -> None:
    config: ExtractionConfig = ExtractionConfig(
        ocr=OcrConfig(
            backend="tesseract", language="eng+fra",
            tesseract_config=TesseractConfig(psm=3)
        )
    )
    result = await extract_file("document.pdf", config=config)
    print(result.content)

asyncio.run(main())
```
