```typescript
import { extractFile } from 'kreuzberg';

const config = {
	tokenReduction: {
		mode: 'moderate',
		preserveImportantWords: true,
	},
};

const result = await extractFile('document.pdf', null, config);
console.log(result.content);
```
