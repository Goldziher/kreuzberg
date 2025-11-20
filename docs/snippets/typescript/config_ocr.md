```typescript
import { ExtractionConfig, OcrConfig, TesseractConfig } from '@kreuzberg/sdk';

const config = new ExtractionConfig({
  ocr: new OcrConfig({
    backend: 'tesseract',
    language: 'eng+fra',
    tesseractConfig: new TesseractConfig({ psm: 3 })
  })
});
```
