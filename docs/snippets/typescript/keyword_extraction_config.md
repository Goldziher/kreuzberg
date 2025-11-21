```typescript
import { ExtractionConfig, KeywordConfig, KeywordAlgorithm } from '@kreuzberg/sdk';

const config = new ExtractionConfig({
  keywords: new KeywordConfig({
    algorithm: KeywordAlgorithm.YAKE,
    maxKeywords: 10,
    minScore: 0.3,
    ngramRange: [1, 3],
    language: 'en'
  })
});
```
