```typescript
import { extractFile } from '@kreuzberg/node';

const config = {
	languageDetection: {
		enabled: true,
		minConfidence: 0.9,
		detectMultiple: true,
	},
};

const result = await extractFile('document.pdf', null, config);
console.log(result.content);
```
