```typescript
import { initWasm, extractBytes } from '@kreuzberg/wasm';

await initWasm();

const config = {
  chunking: {
    maxChars: 1000,
    chunkOverlap: 100
  }
};

const bytes = new Uint8Array(buffer);
const result = await extractBytes(bytes, 'application/pdf', config);

result.chunks?.forEach((chunk, idx) => {
  console.log(`Chunk ${idx}: ${chunk.content.substring(0, 50)}...`);
  console.log(`Tokens: ${chunk.metadata?.token_count}`);
});
```
