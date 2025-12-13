```typescript
import { extractFromFile, initWasm } from '@kreuzberg/wasm';

await initWasm();

const config = {
	ocr: {
		backend: 'tesseract-wasm',
		language: 'eng',
	},
	chunking: {
		maxChars: 1000,
		chunkOverlap: 100,
	},
	enable_language_detection: true,
	enable_quality: true,
};

const fileInput = document.getElementById('file') as HTMLInputElement;
const file = fileInput.files?.[0];

if (file) {
	const result = await extractFromFile(file, file.type, config);

	if (result.chunks) {
		for (const chunk of result.chunks) {
			console.log(`Chunk: ${chunk.content.substring(0, 100)}...`);
		}
	}

	if (result.detectedLanguages) {
		console.log(`Languages: ${result.detectedLanguages.join(', ')}`);
	}
}
```
