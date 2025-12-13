```python title="Python"
import asyncio
from kreuzberg import ExtractionConfig, extract_file

async def main() -> None:
    config: ExtractionConfig = ExtractionConfig(
        enable_quality_processing=True
    )
    result = await extract_file("document.pdf", config=config)

    quality_score: float = result.metadata.get("quality_score", 0.0)
    print(f"Quality score: {quality_score:.2f}")

asyncio.run(main())
```
