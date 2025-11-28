```typescript
import { extractFile } from 'kreuzberg';

const config = {
	languageDetection: {
		enabled: true,
		minConfidence: 0.8,
		detectMultiple: false,
	},
};

const result = await extractFile('document.pdf', null, config);
if (result.detectedLanguages) {
	console.log(`Detected languages: ${result.detectedLanguages.join(', ')}`);
}
```
