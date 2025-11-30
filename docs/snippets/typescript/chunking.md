```typescript
import { extractFile } from '@kreuzberg/node';

const config = {
	chunking: {
		maxChars: 1500,
		maxOverlap: 200,
		embedding: {
			preset: 'quality',
		},
	},
};

const result = await extractFile('document.pdf', null, config);
console.log(`Chunks created: ${result.chunks?.length ?? 0}`);
```
