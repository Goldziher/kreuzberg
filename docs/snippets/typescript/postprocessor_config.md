```typescript
import { extractFile } from 'kreuzberg';

const config = {
	postprocessor: {
		enabled: true,
		enabledProcessors: ['deduplication', 'whitespace_normalization'],
		disabledProcessors: ['mojibake_fix'],
	},
};

const result = await extractFile('document.pdf', null, config);
console.log(result.content);
```
