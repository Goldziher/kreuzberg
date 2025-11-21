```typescript
import { ExtractionConfig, LanguageDetectionConfig } from '@kreuzberg/sdk';

const config = new ExtractionConfig({
  languageDetection: new LanguageDetectionConfig({
    enabled: true,
    minConfidence: 0.8,
    detectMultiple: false
  })
});
```
