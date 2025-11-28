```typescript
import { extractFile } from 'kreuzberg';

const config = {
	chunking: {
		maxChars: 500,
		maxOverlap: 50,
		embedding: {
			preset: 'balanced',
		},
	},
};

const result = await extractFile('research_paper.pdf', null, config);

if (result.chunks) {
	for (const chunk of result.chunks) {
		console.log(`Chunk ${chunk.metadata.chunkIndex + 1}/${chunk.metadata.totalChunks}`);
		console.log(`Position: ${chunk.metadata.charStart}-${chunk.metadata.charEnd}`);
		console.log(`Content: ${chunk.content.slice(0, 100)}...`);
		if (chunk.embedding) {
			console.log(`Embedding: ${chunk.embedding.length} dimensions`);
		}
	}
}
```
