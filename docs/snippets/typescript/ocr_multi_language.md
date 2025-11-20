```typescript
import { extractFileSync, ExtractionConfig, OcrConfig } from 'kreuzberg';

const config = new ExtractionConfig({
  ocr: new OcrConfig({
    backend: 'tesseract',
    language: 'eng+deu+fra'
  })
});

const result = extractFileSync('multilingual.pdf', null, config);
console.log(result.content);
```
