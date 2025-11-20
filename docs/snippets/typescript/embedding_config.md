```typescript
import { ExtractionConfig, ChunkingConfig, EmbeddingConfig, EmbeddingModelType } from '@kreuzberg/sdk';

const config = new ExtractionConfig({
  chunking: new ChunkingConfig({
    maxChars: 1000,
    embedding: new EmbeddingConfig({
      model: EmbeddingModelType.preset('all-mpnet-base-v2'),
      batchSize: 16,
      normalize: true,
      showDownloadProgress: true
    })
  })
});
```
