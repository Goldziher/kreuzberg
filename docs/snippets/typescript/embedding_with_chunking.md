```typescript
import { extractFile } from 'kreuzberg';

const config = {
	chunking: {
		maxChars: 1024,
		maxOverlap: 100,
		embedding: {
			preset: 'balanced',
		},
	},
};

const result = await extractFile('document.pdf', null, config);
console.log(`Chunks: ${result.chunks?.length ?? 0}`);
```
