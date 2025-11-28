```python
import asyncio
from kreuzberg import (
    extract_file,
    ExtractionConfig,
    ChunkingConfig,
    EmbeddingConfig,
    EmbeddingModelType,
)

async def main() -> None:
    config: ExtractionConfig = ExtractionConfig(
        chunking=ChunkingConfig(
            max_chars=500,
            max_overlap=50,
            embedding=EmbeddingConfig(
                model=EmbeddingModelType.preset("balanced"), normalize=True
            ),
        )
    )
    result = await extract_file("research_paper.pdf", config=config)
    for chunk in result.chunks:
        print(f"Content: {chunk.content[:80]}")
        if chunk.embedding:
            print(f"Embedding dimensions: {len(chunk.embedding)}")

asyncio.run(main())
```
