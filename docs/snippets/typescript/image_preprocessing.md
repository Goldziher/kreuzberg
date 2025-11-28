```typescript
import { extractFile } from 'kreuzberg';

const config = {
	ocr: {
		backend: 'tesseract',
		tesseractConfig: {
			psm: 6,
			enableTableDetection: true,
		},
	},
};

const result = await extractFile('document.pdf', null, config);
console.log(result.content);
```
