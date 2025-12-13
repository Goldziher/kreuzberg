import { extractBytes, initWasm } from "@kreuzberg/wasm";
import type { ExtractionConfig } from "@kreuzberg/wasm";

async function extractWithConditionalConfig(fileSize: number) {
  await initWasm();

  const config: ExtractionConfig = {};

  if (fileSize > 10 * 1024 * 1024) {
    config.chunking = {
      maxChars: 500,
      chunkOverlap: 50,
    };
  }

  if (fileSize < 1 * 1024 * 1024) {
    config.images = {
      extractImages: true,
      targetDpi: 300,
    };
  }

  config.ocr = {
    enabled: fileSize < 50 * 1024 * 1024,
  };

  const bytes = new Uint8Array(await fetch("document.pdf").then((r) => r.arrayBuffer()));

  const result = await extractBytes(bytes, "application/pdf", config);

  return result;
}

extractWithConditionalConfig(5 * 1024 * 1024).then((r) => console.log("Done"));
