```python
from kreuzberg import extract_file, ExtractionConfig, ChunkingConfig, EmbeddingConfig, EmbeddingModelType

config = ExtractionConfig(
    chunking=ChunkingConfig(
        max_chars=500,
        max_overlap=50,
        embedding=EmbeddingConfig(
            model=EmbeddingModelType.preset("balanced"),
            normalize=True
        )
    )
)

result = extract_file("research_paper.pdf", config=config)

for chunk in result.chunks:
    print(f"Chunk {chunk.metadata['chunk_index'] + 1}/{chunk.metadata['total_chunks']}")
    print(f"Position: {chunk.metadata['char_start']}-{chunk.metadata['char_end']}")
    print(f"Content: {chunk.content[:100]}...")
    if chunk.embedding:
        print(f"Embedding: {len(chunk.embedding)} dimensions")
```
