```typescript
import { extractFileSync } from 'kreuzberg';

const result = extractFileSync('document.pdf');

console.log(result.content);
console.log(`Tables: ${result.tables.length}`);
console.log(`Metadata: ${JSON.stringify(result.metadata)}`);
```
