```typescript
import { extractFile, ExtractionConfig, ChunkingConfig, EmbeddingConfig } from '@kreuzberg/sdk';
import { ChromaClient } from 'chromadb';

const config = new ExtractionConfig({
  chunking: new ChunkingConfig({
    maxChars: 512,
    maxOverlap: 50,
    embedding: new EmbeddingConfig({ model: 'balanced', normalize: true })
  })
});

const result = await extractFile('document.pdf', { config });

const client = new ChromaClient();
const collection = await client.createCollection({ name: 'documents' });

for (let i = 0; i < result.chunks.length; i++) {
  const chunk = result.chunks[i];
  await collection.add({
    ids: [`doc_chunk_${i}`],
    embeddings: [chunk.embedding],
    documents: [chunk.content],
    metadatas: [chunk.metadata]
  });
}

// Semantic search
const queryResult = await extractFile('query.txt', { config });
const results = await collection.query({
  queryEmbeddings: [queryResult.chunks[0].embedding],
  nResults: 5
});
```
