```typescript
import { ExtractionConfig, PostProcessorConfig } from '@kreuzberg/sdk';

const config = new ExtractionConfig({
  postprocessor: new PostProcessorConfig({
    enabled: true,
    enabledProcessors: ['deduplication', 'whitespace_normalization'],
    disabledProcessors: ['mojibake_fix'],
  }),
});
```
