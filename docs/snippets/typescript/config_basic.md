```typescript
import { extractFile, ExtractionConfig } from '@kreuzberg/sdk';

const config = new ExtractionConfig({
  useCache: true,
  enableQualityProcessing: true
});

const result = await extractFile('document.pdf', { config });
```
