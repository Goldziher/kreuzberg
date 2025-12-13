```typescript
import { extractFromFile, initWasm } from '@kreuzberg/wasm';

await initWasm();

const fileInput = document.getElementById('file') as HTMLInputElement;
const file = fileInput.files?.[0];

if (file) {
	try {
		const result = await extractFromFile(file);
		console.log(result.content);
	} catch (error) {
		if (error instanceof Error) {
			console.error(`Extraction error: ${error.message}`);
		} else {
			throw error;
		}
	}
}
```
