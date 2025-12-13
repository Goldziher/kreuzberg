```python title="Python"
import asyncio
from kreuzberg import extract_file, ExtractionConfig, LanguageDetectionConfig

async def main() -> None:
    config: ExtractionConfig = ExtractionConfig(
        language_detection=LanguageDetectionConfig(
            enabled=True,
            min_confidence=0.7,
            detect_multiple=True
        )
    )
    result = await extract_file("multilingual_document.pdf", config=config)
    languages: list[str] = result.detected_languages or []
    print(f"Detected {len(languages)} languages: {languages}")

asyncio.run(main())
```
