```typescript
import { initWasm, extractBytes } from '@kreuzberg/wasm';

await initWasm();

const config = {
  use_cache: true,
  enable_quality_processing: true,
  ocr: {
    backend: 'tesseract-wasm',
    language: 'eng'
  }
};

const bytes = new Uint8Array(buffer);
const result = await extractBytes(bytes, 'application/pdf', config);
console.log(result.content);
```
