```typescript title="TypeScript"
import { extractFile, ExtractionConfig } from '@kreuzberg/node';

const config = ExtractionConfig.discover();
const result = await extractFile('document.pdf', null, config);
console.log(result.content);
```
