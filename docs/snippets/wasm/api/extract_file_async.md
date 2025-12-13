```typescript
import { initWasm, extractFile } from '@kreuzberg/wasm';

await initWasm();

// Extract from file path (async)
const result = await extractFile('document.pdf');

console.log(result.content);
console.log(`Tables: ${result.tables.length}`);
console.log(`Metadata: ${JSON.stringify(result.metadata)}`);
```
