```typescript title="TypeScript"
import { extractFileSync } from '@kreuzberg/node';

const config = {
	ocr: {
		backend: 'tesseract',
		language: 'eng',
	},
};

const result = extractFileSync('scanned.pdf', null, config);
console.log(result.content);
```
