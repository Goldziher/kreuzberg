```typescript
import { initWasm, extractBytes } from '@kreuzberg/wasm';

try {
	await initWasm();
	const bytes = new Uint8Array(buffer);
	const result = await extractBytes(bytes, 'application/pdf');
	console.log(result.content);
} catch (error) {
	if (error instanceof Error) {
		console.error(`Extraction error: ${error.message}`);
	} else {
		throw error;
	}
}
```
