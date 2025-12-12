```python title="Python"
import asyncio
from kreuzberg import ExtractionConfig, OcrConfig, TesseractConfig, extract_file

async def main() -> None:
    config: ExtractionConfig = ExtractionConfig(
        ocr=OcrConfig(
            language="eng+fra+deu",
            tesseract_config=TesseractConfig(
                psm=6,
                oem=1,
                min_confidence=0.8,
                enable_table_detection=True,
            ),
        )
    )
    result = await extract_file("document.pdf", config=config)
    print(f"Content: {result.content[:100]}")

asyncio.run(main())
```
