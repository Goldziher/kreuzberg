```typescript title="TypeScript"
import { extractFile } from '@kreuzberg/node';

const config = {
	chunking: {
		maxChars: 512,
		maxOverlap: 50,
		embedding: {
			preset: 'balanced',
		},
	},
};

const result = await extractFile('document.pdf', null, config);

if (result.chunks) {
	for (const chunk of result.chunks) {
		console.log(`Chunk: ${chunk.content.slice(0, 100)}...`);
		if (chunk.embedding) {
			console.log(`Embedding dims: ${chunk.embedding.length}`);
		}
	}
}
```
