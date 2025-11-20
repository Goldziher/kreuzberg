```typescript
import { extractFileSync, ExtractionConfig, OcrConfig, PdfConfig } from 'kreuzberg';

const config = new ExtractionConfig({
  ocr: new OcrConfig({ backend: 'tesseract' }),
  pdf: new PdfConfig({ dpi: 300 })
});

const result = extractFileSync('scanned.pdf', null, config);
```
