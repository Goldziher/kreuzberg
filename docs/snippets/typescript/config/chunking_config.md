```typescript title="TypeScript"
import { extractFile } from '@kreuzberg/node';

const config = {
	chunking: {
		maxChars: 1000,
		maxOverlap: 200,
	},
};

const result = await extractFile('document.pdf', null, config);
console.log(`Total chunks: ${result.chunks?.length ?? 0}`);
```
