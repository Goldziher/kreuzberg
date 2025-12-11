```python
import asyncio
from kreuzberg import ExtractionConfig, ChunkingConfig, extract_file

async def main() -> None:
    config: ExtractionConfig = ExtractionConfig(
        chunking=ChunkingConfig(
            max_chars=1000,
            max_overlap=200,
            separator="sentence"
        )
    )
    result = await extract_file("document.pdf", config=config)
    print(f"Chunks: {len(result.chunks or [])}")
    for chunk in result.chunks or []:
        print(f"Length: {len(chunk.content)}")

asyncio.run(main())
```
