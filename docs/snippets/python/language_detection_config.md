```python
import asyncio
from kreuzberg import ExtractionConfig, LanguageDetectionConfig, extract_file

async def main() -> None:
    config: ExtractionConfig = ExtractionConfig(
        language_detection=LanguageDetectionConfig(
            enabled=True, min_confidence=0.8, detect_multiple=False
        )
    )
    result = await extract_file("document.pdf", config=config)
    print(f"Languages: {result.detected_languages}")

asyncio.run(main())
```
