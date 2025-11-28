```typescript
import { extractFileSync, type ExtractionConfig } from 'kreuzberg';

const config: ExtractionConfig = {
	useCache: true,
	ocr: {
		backend: 'tesseract',
		language: 'eng+deu',
		tesseractConfig: {
			psm: 6,
		},
	},
	chunking: {
		maxChars: 1000,
		maxOverlap: 200,
	},
	enableQualityProcessing: true,
};

const result = extractFileSync('document.pdf', null, config);
console.log(`Content length: ${result.content.length}`);
```
