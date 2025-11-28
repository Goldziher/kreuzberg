```typescript
import { extractFileSync } from 'kreuzberg';

const config = {
	ocr: {
		backend: 'tesseract',
	},
	pdfOptions: {
		extractImages: true,
	},
};

const result = extractFileSync('scanned.pdf', null, config);
console.log(result.content);
```
