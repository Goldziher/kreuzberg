```typescript
import { extractFile } from '@kreuzberg/node';

const config = {
	useCache: true,
	enableQualityProcessing: true,
};

const result = await extractFile('document.pdf', null, config);
console.log(result.content);
```
