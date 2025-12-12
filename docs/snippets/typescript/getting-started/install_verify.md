```typescript title="TypeScript"
import { getVersion, extractFileSync } from '@kreuzberg/node';

const version = getVersion();
console.log(`Kreuzberg version: ${version}`);

const result = extractFileSync('document.pdf');
console.log(`Extraction successful: ${result.success}`);
```
