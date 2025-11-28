```typescript
import { extractFile } from 'kreuzberg';

const config = {
	tokenReduction: {
		mode: 'moderate',
		preserveImportantWords: true,
	},
};

const result = await extractFile('verbose_document.pdf', null, config);
console.log(`Content length: ${result.content.length}`);
console.log(`Metadata: ${JSON.stringify(result.metadata)}`);
```
