```typescript
import { extractFile } from '@kreuzberg/node';

const config = {
	keywords: {
		algorithm: 'yake',
		maxKeywords: 10,
		minScore: 0.3,
		ngramRange: [1, 3],
		language: 'en',
	},
};

const result = await extractFile('document.pdf', null, config);
console.log(`Content: ${result.content}`);
```
