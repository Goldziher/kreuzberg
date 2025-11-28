```python
import asyncio
from kreuzberg import ExtractionConfig, extract_file

async def main() -> None:
    config: ExtractionConfig = ExtractionConfig(
        enable_quality_processing=True
    )
    result = await extract_file("document.pdf", config=config)
    print(f"Quality score: {result.metadata.get('quality_score', 0)}")

asyncio.run(main())
```
