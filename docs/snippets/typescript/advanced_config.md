```typescript
import {
	extractFileSync,
	ExtractionConfig,
	OcrConfig,
	ChunkingConfig,
	TokenReductionConfig,
	LanguageDetectionConfig,
} from "kreuzberg";

const config = new ExtractionConfig({
	// Enable OCR
	ocr: new OcrConfig({
		backend: "tesseract",
		language: "eng+deu", // Multiple languages
	}),

	// Enable chunking for LLM processing
	chunking: new ChunkingConfig({
		maxChunkSize: 1000,
		overlap: 100,
	}),

	// Enable token reduction
	tokenReduction: new TokenReductionConfig({
		enabled: true,
		targetReduction: 0.3, // Reduce by 30%
	}),

	// Enable language detection
	languageDetection: new LanguageDetectionConfig({
		enabled: true,
		detectMultiple: true,
	}),

	// Enable caching
	useCache: true,

	// Enable quality processing
	enableQualityProcessing: true,
});

const result = extractFileSync("document.pdf", null, config);

// Access chunks
for (const chunk of result.chunks) {
	console.log(`Chunk: ${chunk.text.substring(0, 100)}...`);
}

// Access detected languages
if (result.detectedLanguages) {
	console.log(`Languages: ${result.detectedLanguages}`);
}
```
