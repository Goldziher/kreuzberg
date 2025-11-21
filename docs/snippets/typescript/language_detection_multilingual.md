```typescript
import { extractFile, ExtractionConfig, LanguageDetectionConfig } from '@kreuzberg/sdk';

const config = new ExtractionConfig({
  languageDetection: new LanguageDetectionConfig({
    enabled: true,
    minConfidence: 0.8,
    detectMultiple: true
  })
});

const result = await extractFile('multilingual_document.pdf', { config });

console.log(`Detected languages: ${result.detectedLanguages}`);
// Output: ['eng', 'fra', 'deu']
```
