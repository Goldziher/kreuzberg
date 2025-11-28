```typescript
import { extractFile } from 'kreuzberg';

const config = {
	enableQualityProcessing: true,
};

const result = await extractFile('scanned_document.pdf', null, config);
console.log(`Content length: ${result.content.length} characters`);
console.log(`Metadata: ${JSON.stringify(result.metadata)}`);
```
