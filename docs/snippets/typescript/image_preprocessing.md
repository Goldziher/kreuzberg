```typescript
import { ExtractionConfig, OcrConfig, TesseractConfig, ImagePreprocessingConfig } from '@kreuzberg/sdk';

const config = new ExtractionConfig({
  ocr: new OcrConfig({
    tesseractConfig: new TesseractConfig({
      preprocessing: new ImagePreprocessingConfig({
        targetDpi: 300,
        denoise: true,
        deskew: true,
        contrastEnhance: true,
        binarizationMethod: 'otsu'
      })
    })
  })
});
```
