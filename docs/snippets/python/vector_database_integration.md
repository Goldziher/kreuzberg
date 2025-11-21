```python
from kreuzberg import extract_file, ExtractionConfig, ChunkingConfig, EmbeddingConfig, EmbeddingModelType
import chromadb

config = ExtractionConfig(
    chunking=ChunkingConfig(
        max_chars=512,
        max_overlap=50,
        embedding=EmbeddingConfig(
            model=EmbeddingModelType.preset("balanced"),
            normalize=True
        )
    )
)

result = extract_file("document.pdf", config=config)

client = chromadb.Client()
collection = client.create_collection("documents")

for i, chunk in enumerate(result.chunks):
    collection.add(
        ids=[f"doc_chunk_{i}"],
        embeddings=[chunk.embedding],
        documents=[chunk.content],
        metadatas=[chunk.metadata]
    )

# Semantic search
query_result = extract_file("query.txt", config=config)
results = collection.query(
    query_embeddings=[query_result.chunks[0].embedding],
    n_results=5
)
```
