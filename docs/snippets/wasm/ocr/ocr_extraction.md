```typescript
import { enableOcr, extractFromFile, initWasm } from '@kreuzberg/wasm';

await initWasm();
await enableOcr();

const fileInput = document.getElementById('file') as HTMLInputElement;
const file = fileInput.files?.[0];

if (file) {
	const result = await extractFromFile(file, file.type, {
		ocr: {
			backend: 'tesseract-wasm',
			language: 'eng',
		},
	});
	console.log(result.content);
}
```
