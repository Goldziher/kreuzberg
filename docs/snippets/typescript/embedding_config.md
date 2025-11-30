```typescript
import { extractFile } from '@kreuzberg/node';

const config = {
	chunking: {
		maxChars: 1000,
		embedding: {
			preset: 'quality',
		},
	},
};

const result = await extractFile('document.pdf', null, config);
if (result.chunks && result.chunks.length > 0) {
	console.log(`Chunk embeddings: ${result.chunks[0].embedding?.length ?? 0} dimensions`);
}
```
