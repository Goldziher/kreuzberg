```typescript
import { extractFileSync } from 'kreuzberg';

const config = {
	ocr: {
		backend: 'tesseract',
		language: 'eng',
	},
};

const result = extractFileSync('scanned.pdf', null, config);
console.log(result.content);
```
