```typescript
import { extractFile, ExtractionConfig, TokenReductionConfig } from '@kreuzberg/sdk';

const config = new ExtractionConfig({
  tokenReduction: new TokenReductionConfig({
    mode: 'moderate',
    preserveMarkdown: true
  })
});

const result = await extractFile('verbose_document.pdf', { config });

const originalTokens = result.metadata.original_token_count;
const reducedTokens = result.metadata.token_count;
const reductionRatio = result.metadata.token_reduction_ratio;

console.log(`Reduced from ${originalTokens} to ${reducedTokens} tokens`);
console.log(`Reduction: ${reductionRatio * 100}%`);
```
