import { extractBytes, initWasm } from "@kreuzberg/wasm";
import type { ExtractionConfig } from "@kreuzberg/wasm";

async function extractWithFullConfig() {
  await initWasm();

  const bytes = new Uint8Array(await fetch("complex.pdf").then((r) => r.arrayBuffer()));

  const config: ExtractionConfig = {
    ocr: {
      backend: "tesseract-wasm",
      language: "deu",
    },
    chunking: {
      maxChars: 1000,
      chunkOverlap: 200,
    },
    images: {
      extractImages: true,
      targetDpi: 200,
    },
  };

  const result = await extractBytes(bytes, "application/pdf", config);

  console.log("=== Extraction Results ===");
  console.log(`Content: ${result.content.length} chars`);
  console.log(`Chunks: ${result.chunks?.length ?? 0}`);
  console.log(`Images: ${result.images?.length ?? 0}`);
  console.log(`Tables: ${result.tables.length}`);
  console.log(`Languages: ${result.detectedLanguages?.join(", ")}`);
}

extractWithFullConfig().catch(console.error);
