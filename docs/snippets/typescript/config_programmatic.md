```ts
import { extractFileSync, type ExtractionConfig } from "@kreuzberg/node";

const config: ExtractionConfig = {
  useCache: true,
  ocr: {
    backend: "tesseract",
    language: "eng+deu",
    tesseract: { psm: 6 },
  },
  chunking: {
    maxChars: 1000,
    maxOverlap: 200,
  },
  enableQualityProcessing: true,
};

const result = extractFileSync("document.pdf", undefined, config);
console.log(`Content length: ${result.content.length}`);
```
