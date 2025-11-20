```typescript
import { ExtractionConfig, PdfConfig } from '@kreuzberg/sdk';

const config = new ExtractionConfig({
  pdfOptions: new PdfConfig({
    extractImages: true,
    extractMetadata: true,
    passwords: ['password1', 'password2']
  })
});
```
