```typescript
import { initWasm, batchExtractFiles } from '@kreuzberg/wasm';

await initWasm();

const files = [
  new File(['content1'], 'doc1.pdf', { type: 'application/pdf' }),
  new File(['content2'], 'doc2.pdf', { type: 'application/pdf' })
];

const results = await batchExtractFiles(files);

results.forEach((result, index) => {
  console.log(`Document ${index + 1}: ${result.content.substring(0, 100)}...`);
});
```
