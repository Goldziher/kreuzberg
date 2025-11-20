```typescript
import { extractFileSync, ExtractionConfig, OcrConfig } from 'kreuzberg';

const config = new ExtractionConfig({
  ocr: new OcrConfig({ backend: 'tesseract' }),
  forceOcr: true
});

const result = extractFileSync('document.pdf', null, config);
console.log(result.content);
```
