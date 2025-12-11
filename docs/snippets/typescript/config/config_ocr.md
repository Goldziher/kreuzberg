```typescript
import { extractFile } from '@kreuzberg/node';

const config = {
	ocr: {
		backend: 'tesseract',
		language: 'eng+fra',
		tesseractConfig: {
			psm: 3,
		},
	},
};

const result = await extractFile('document.pdf', null, config);
console.log(result.content);
```
