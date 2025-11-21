```typescript
import { ExtractionConfig, TokenReductionConfig } from '@kreuzberg/sdk';

const config = new ExtractionConfig({
  tokenReduction: new TokenReductionConfig({
    mode: 'moderate',
    preserveMarkdown: true,
    preserveCode: true,
    languageHint: 'eng'
  })
});
```
