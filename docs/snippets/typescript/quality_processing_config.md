```typescript
import { extractFile } from 'kreuzberg';

const config = {
	enableQualityProcessing: true,
};

const result = await extractFile('document.pdf', null, config);
console.log(result.content);
```
