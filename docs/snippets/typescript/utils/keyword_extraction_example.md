```typescript title="TypeScript"
import { extractFile } from '@kreuzberg/node';

const config = {
	keywords: {
		algorithm: 'yake',
		maxKeywords: 10,
		minScore: 0.3,
	},
};

const result = await extractFile('research_paper.pdf', null, config);
console.log(`Content length: ${result.content.length}`);
console.log(`Metadata: ${JSON.stringify(result.metadata)}`);
```
