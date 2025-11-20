```typescript
import { ExtractionConfig, OcrConfig, TesseractConfig } from '@kreuzberg/sdk';

const config = new ExtractionConfig({
  ocr: new OcrConfig({
    language: 'eng+fra+deu',
    tesseractConfig: new TesseractConfig({
      psm: 6,
      oem: 1,
      minConfidence: 0.8,
      tesseditCharWhitelist: 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789 .,!?',
      enableTableDetection: true
    })
  })
});
```
