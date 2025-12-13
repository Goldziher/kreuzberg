```typescript
import { extractFromFile, initWasm } from '@kreuzberg/wasm';

await initWasm();

const fileInput = document.getElementById('file') as HTMLInputElement;
const file = fileInput.files?.[0];

if (file) {
	const result = await extractFromFile(file);
	console.log(`Metadata: ${JSON.stringify(result.metadata)}`);

	if (result.metadata.page_count) {
		console.log(`Pages: ${result.metadata.page_count}`);
	}

	if (result.metadata.title) {
		console.log(`Title: ${result.metadata.title}`);
	}
}
```
