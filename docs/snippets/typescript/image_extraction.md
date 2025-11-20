```typescript
import { ExtractionConfig, ImageExtractionConfig } from '@kreuzberg/sdk';

const config = new ExtractionConfig({
  images: new ImageExtractionConfig({
    extractImages: true,
    targetDpi: 200,
    maxImageDimension: 2048,
    autoAdjustDpi: true
  })
});
```
