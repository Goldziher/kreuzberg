```python title="Python"
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
            max_chars=512,
            max_overlap=50,
            embedding=EmbeddingConfig(
                model=EmbeddingModelType.preset("balanced"), normalize=True
            ),
        )
    )
    result = await extract_file("document.pdf", config=config)
    chunks = result.chunks or []
    for i, chunk in enumerate(chunks):
        chunk_id: str = f"doc_chunk_{i}"
        print(f"Chunk {chunk_id}: {chunk.content[:50]}")

asyncio.run(main())
```
