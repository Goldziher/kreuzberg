```python
import asyncio
from kreuzberg import ExtractionConfig, extract_file

async def main() -> None:
    config: ExtractionConfig = ExtractionConfig()
    result = await extract_file("document.pdf", config=config)

    content: str = result.content
    content_preview: str = content[:100]

    print(f"Content preview: {content_preview}")
    print(f"Total length: {len(content)}")

asyncio.run(main())
```
