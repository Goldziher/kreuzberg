import { extractBytes, initWasm } from "@kreuzberg/wasm";
import type { ExtractionConfig } from "@kreuzberg/wasm";

async function extractWithChunking() {
  await initWasm();

  const bytes = new Uint8Array(await fetch("large.pdf").then((r) => r.arrayBuffer()));

  const config: ExtractionConfig = {
    chunking: {
      maxChars: 500,
      chunkOverlap: 50,
    },
  };

  const result = await extractBytes(bytes, "application/pdf", config);

  if (result.chunks) {
    console.log(`Document split into ${result.chunks.length} chunks`);

    result.chunks.forEach((chunk, index) => {
      console.log(`Chunk ${index}: ${chunk.content.length} chars`);
      console.log(`Position: ${chunk.metadata.charStart}-${chunk.metadata.charEnd}`);
    });
  }
}

extractWithChunking().catch(console.error);
