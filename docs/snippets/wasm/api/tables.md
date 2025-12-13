```typescript
import { extractFromFile, initWasm } from '@kreuzberg/wasm';

await initWasm();

const fileInput = document.getElementById('file') as HTMLInputElement;
const file = fileInput.files?.[0];

if (file) {
	const result = await extractFromFile(file);

	for (const table of result.tables) {
		console.log(`Table with ${table.cells.length} rows`);
		console.log(`Page: ${table.pageNumber}`);
		console.log(table.markdown);
	}
}
```
