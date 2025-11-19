```typescript
import { extractFileSync, ExtractionConfig } from 'kreuzberg';

const result = extractFileSync('document.pdf', null, new ExtractionConfig());

console.log(result.content);
console.log(`Tables: ${result.tables.length}`);
console.log(`Metadata: ${JSON.stringify(result.metadata)}`);
```
