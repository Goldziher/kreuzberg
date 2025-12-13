```python title="Python"
import asyncio
from kreuzberg import ExtractionConfig, LanguageDetectionConfig, extract_file

async def main() -> None:
    config: ExtractionConfig = ExtractionConfig(
        language_detection=LanguageDetectionConfig(
            enabled=True,
            min_confidence=0.85,
            detect_multiple=False
        )
    )
    result = await extract_file("document.pdf", config=config)
    if result.detected_languages:
        print(f"Primary language: {result.detected_languages[0]}")
    print(f"Content length: {len(result.content)} chars")

asyncio.run(main())
```
