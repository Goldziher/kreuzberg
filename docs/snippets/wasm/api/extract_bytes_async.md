```typescript
import { initWasm, extractBytes } from '@kreuzberg/wasm';

await initWasm();

const bytes = new Uint8Array(buffer);
const result = await extractBytes(bytes, 'application/pdf');

console.log(result.content);
console.log(`Tables: ${result.tables.length}`);
console.log(`Metadata: ${JSON.stringify(result.metadata)}`);
```
