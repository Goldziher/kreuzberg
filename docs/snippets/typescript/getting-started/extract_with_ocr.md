```typescript title="TypeScript"
import { extractFileSync } from '@kreuzberg/node';

const config = {
	forceOcr: true,
	ocr: {
		backend: 'tesseract',
		language: 'eng',
	},
};

const result = extractFileSync('scanned.pdf', null, config);

console.log(result.content);
console.log(`Detected Languages: ${result.detectedLanguages?.join(', ') ?? 'none'}`);
```
