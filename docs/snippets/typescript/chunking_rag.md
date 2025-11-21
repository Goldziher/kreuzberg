```typescript
import { extractFile, ExtractionConfig, ChunkingConfig, EmbeddingConfig } from '@kreuzberg/sdk';

const config = new ExtractionConfig({
  chunking: new ChunkingConfig({
    maxChars: 500,
    maxOverlap: 50,
    embedding: new EmbeddingConfig({
      model: 'balanced',
      normalize: true
    })
  })
});

const result = await extractFile('research_paper.pdf', { config });

for (const chunk of result.chunks) {
  console.log(`Chunk ${chunk.metadata.chunkIndex + 1}/${chunk.metadata.totalChunks}`);
  console.log(`Position: ${chunk.metadata.charStart}-${chunk.metadata.charEnd}`);
  console.log(`Content: ${chunk.content.slice(0, 100)}...`);
  if (chunk.embedding) {
    console.log(`Embedding: ${chunk.embedding.length} dimensions`);
  }
}
```
