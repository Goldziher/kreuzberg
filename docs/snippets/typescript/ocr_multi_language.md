```typescript
import { extractFileSync } from 'kreuzberg';

const config = {
	ocr: {
		backend: 'tesseract',
		language: 'eng+deu+fra',
	},
};

const result = extractFileSync('multilingual.pdf', null, config);
console.log(result.content);
```
