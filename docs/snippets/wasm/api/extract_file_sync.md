```typescript
import { initWasm, extractFile } from '@kreuzberg/wasm';

// Initialize WASM module once at app startup
await initWasm();

// Extract from file path (Node.js/Deno/Bun only)
const result = await extractFile('document.pdf');

console.log(result.content);
console.log(`Tables: ${result.tables.length}`);
console.log(`Metadata: ${JSON.stringify(result.metadata)}`);
```
