```typescript title="TypeScript"
import { extractFileSync } from '@kreuzberg/node';

const config = {
	useCache: true,
	enableQualityProcessing: true,
};

const result = extractFileSync('document.pdf', null, config);

console.log(result.content);
console.log(`MIME Type: ${result.mimeType}`);
```
