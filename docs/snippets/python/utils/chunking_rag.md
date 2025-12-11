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
                model=EmbeddingModelType.preset("balanced"),
                normalize=True,
                batch_size=16
            )
        )
    )
    result = await extract_file("research_paper.pdf", config=config)

    chunks_with_embeddings: list = []
    for chunk in result.chunks or []:
        if chunk.embedding:
            chunks_with_embeddings.append({
                "content": chunk.content[:100],
                "embedding_dims": len(chunk.embedding)
            })

    print(f"Chunks with embeddings: {len(chunks_with_embeddings)}")

asyncio.run(main())
```
