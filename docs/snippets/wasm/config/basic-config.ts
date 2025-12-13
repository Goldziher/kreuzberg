import { extractBytes, initWasm } from "@kreuzberg/wasm";
import type { ExtractionConfig } from "@kreuzberg/wasm";

async function extractWithConfig() {
  await initWasm();

  const bytes = new Uint8Array(await fetch("document.pdf").then((r) => r.arrayBuffer()));

  const config: ExtractionConfig = {
    ocr: {
      backend: "tesseract-wasm",
      language: "eng",
    },
    images: {
      extractImages: true,
      targetDpi: 200,
    },
    chunking: {
      maxChars: 1000,
      chunkOverlap: 100,
    },
  };

  const result = await extractBytes(bytes, "application/pdf", config);
  console.log("Extraction complete");
  console.log("Content length:", result.content.length);
}

extractWithConfig().catch(console.error);
