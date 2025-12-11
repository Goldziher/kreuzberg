```python
import asyncio
from kreuzberg import ExtractionConfig, PostProcessorConfig, extract_file

async def main() -> None:
    config: ExtractionConfig = ExtractionConfig(
        postprocessor=PostProcessorConfig(
            enabled=True,
            enabled_processors=["deduplication"],
        )
    )
    result = await extract_file("document.pdf", config=config)
    print(f"Content: {result.content[:100]}")

asyncio.run(main())
```
