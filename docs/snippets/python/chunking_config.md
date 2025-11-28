```python
import asyncio
from kreuzberg import ExtractionConfig, ChunkingConfig, extract_file

async def main() -> None:
    config: ExtractionConfig = ExtractionConfig(
        chunking=ChunkingConfig(max_chars=1000, max_overlap=200)
    )
    result = await extract_file("document.pdf", config=config)
    print(f"Chunks: {len(result.chunks)}")

asyncio.run(main())
```
