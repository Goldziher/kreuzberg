```typescript
import { ExtractionConfig, ChunkingConfig, EmbeddingConfig } from '@kreuzberg/sdk';

const config = new ExtractionConfig({
  chunking: new ChunkingConfig({
    maxChars: 1024,
    maxOverlap: 100,
    embedding: new EmbeddingConfig({
      model: 'balanced',
      normalize: true,
      batchSize: 32,
      showDownloadProgress: false
    })
  })
});
```
