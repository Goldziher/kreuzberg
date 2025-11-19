```typescript
import { extractFileSync, ExtractionConfig, OcrConfig } from 'kreuzberg';

const config = new ExtractionConfig({
    ocr: new OcrConfig({
        backend: 'tesseract',
        language: 'eng'
    })
});

const result = extractFileSync('scanned.pdf', null, config);
console.log(result.content);
```
