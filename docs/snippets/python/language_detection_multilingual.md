```python
import asyncio
from kreuzberg import extract_file, ExtractionConfig, LanguageDetectionConfig

async def main() -> None:
    config: ExtractionConfig = ExtractionConfig(
        language_detection=LanguageDetectionConfig(
            enabled=True, min_confidence=0.8, detect_multiple=True
        )
    )
    result = await extract_file("multilingual_document.pdf", config=config)
    langs: list[str] = result.detected_languages or []
    print(f"Detected languages: {langs}")

asyncio.run(main())
```
