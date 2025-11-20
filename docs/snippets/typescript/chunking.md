```typescript
import { ExtractionConfig, ChunkingConfig, EmbeddingConfig, EmbeddingModelType } from '@kreuzberg/sdk';

const config = new ExtractionConfig({
  chunking: new ChunkingConfig({
    maxChars: 1500,
    maxOverlap: 200,
    embedding: new EmbeddingConfig({
      model: new EmbeddingModelType({
        type: 'preset',
        name: 'text-embedding-all-minilm-l6-v2'
      })
    })
  })
});
```
