```typescript
import { extractFile, ExtractionConfig, KeywordConfig, KeywordAlgorithm } from '@kreuzberg/sdk';

const config = new ExtractionConfig({
  keywords: new KeywordConfig({
    algorithm: KeywordAlgorithm.YAKE,
    maxKeywords: 10,
    minScore: 0.3
  })
});

const result = await extractFile('research_paper.pdf', { config });

const keywords = result.metadata.keywords || [];
for (const kw of keywords) {
  console.log(`${kw.text}: ${kw.score.toFixed(3)}`);
}
```
