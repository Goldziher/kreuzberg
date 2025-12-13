```typescript
import { extractFromFile, initWasm } from '@kreuzberg/wasm';

await initWasm();

const fileInputs = document.getElementById('files') as HTMLInputElement;
const files = Array.from(fileInputs.files || []);

const results = await Promise.all(
	files.map((file) => extractFromFile(file))
);

results.forEach((result, i) => {
	console.log(`File ${i + 1}: ${result.content.length} characters`);
});
```
