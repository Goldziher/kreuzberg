```typescript
import { extractFile } from '@kreuzberg/node';

const config = {
	languageDetection: {
		enabled: true,
		minConfidence: 0.8,
		detectMultiple: true,
	},
};

const result = await extractFile('multilingual_document.pdf', null, config);
if (result.detectedLanguages) {
	console.log(`Detected languages: ${result.detectedLanguages.join(', ')}`);
}
```
