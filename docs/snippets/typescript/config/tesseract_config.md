```typescript title="TypeScript"
import { extractFile } from '@kreuzberg/node';

const config = {
	ocr: {
		backend: 'tesseract',
		language: 'eng+fra+deu',
		tesseractConfig: {
			psm: 6,
			tesseditCharWhitelist: 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789 .,!?',
			enableTableDetection: true,
		},
	},
};

const result = await extractFile('document.pdf', null, config);
console.log(result.content);
```
