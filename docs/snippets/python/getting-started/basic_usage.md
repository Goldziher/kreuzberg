```python title="Python"
import asyncio
from kreuzberg import extract_file, ExtractionConfig

async def main() -> None:
    config = ExtractionConfig(
        use_cache=True,
        enable_quality_processing=True
    )
    result = await extract_file("document.pdf", config=config)
    print(result.content)

asyncio.run(main())
```
