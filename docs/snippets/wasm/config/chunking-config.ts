import { extractBytes, initWasm } from "@kreuzberg/wasm";
import type { ExtractionConfig } from "@kreuzberg/wasm";

async function extractWithChunking() {
  await initWasm();

  const bytes = new Uint8Array(await fetch("book.pdf").then((r) => r.arrayBuffer()));

  const config: ExtractionConfig = {
    chunking: {
      maxChars: 800,
      chunkOverlap: 150,
      splitOnNewlines: true,
      splitOnSentences: true,
    },
  };

  const result = await extractBytes(bytes, "application/pdf", config);

  if (result.chunks) {
    console.log(`Total chunks: ${result.chunks.length}`);

    result.chunks.slice(0, 3).forEach((chunk, i) => {
      console.log(`\nChunk ${i}:`);
      console.log(`Chars: ${chunk.metadata.charStart}-${chunk.metadata.charEnd}`);
      console.log(`Content: ${chunk.content.substring(0, 100)}...`);
    });
  }
}

extractWithChunking().catch(console.error);
