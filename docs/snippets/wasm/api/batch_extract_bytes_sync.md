```typescript
import { initWasm, batchExtractBytes } from '@kreuzberg/wasm';

await initWasm();

const dataList = [
  new Uint8Array(buffer1),
  new Uint8Array(buffer2)
];

const mimeTypes = [
  'application/pdf',
  'application/pdf'
];

const results = await batchExtractBytes(dataList, mimeTypes);

results.forEach((result, index) => {
  console.log(`Document ${index + 1}: ${result.content.substring(0, 100)}...`);
});
```
