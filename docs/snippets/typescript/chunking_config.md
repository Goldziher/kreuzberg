```typescript
import { ExtractionConfig, ChunkingConfig } from '@kreuzberg/sdk';

const config = new ExtractionConfig({
  chunking: new ChunkingConfig({
    maxChars: 1000,
    maxOverlap: 200
  })
});
```
