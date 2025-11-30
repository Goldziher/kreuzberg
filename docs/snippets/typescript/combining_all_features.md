```typescript
import { extractFile } from '@kreuzberg/node';

const config = {
	enableQualityProcessing: true,
	languageDetection: {
		enabled: true,
		detectMultiple: true,
	},
	tokenReduction: {
		mode: 'moderate',
		preserveImportantWords: true,
	},
	chunking: {
		maxChars: 512,
		maxOverlap: 50,
		embedding: {
			preset: 'balanced',
		},
	},
	keywords: {
		algorithm: 'yake',
		maxKeywords: 10,
	},
};

const result = await extractFile('document.pdf', null, config);

console.log(`Content length: ${result.content.length}`);
if (result.detectedLanguages) {
	console.log(`Languages: ${result.detectedLanguages.join(', ')}`);
}
if (result.chunks && result.chunks.length > 0) {
	console.log(`Chunks: ${result.chunks.length}`);
}
```
