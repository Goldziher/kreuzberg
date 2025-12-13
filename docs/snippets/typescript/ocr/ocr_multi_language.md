```typescript title="TypeScript"
import { extractFileSync } from '@kreuzberg/node';

const config = {
	ocr: {
		backend: 'tesseract',
		language: 'eng+deu+fra',
	},
};

const result = extractFileSync('multilingual.pdf', null, config);
console.log(result.content);
```
