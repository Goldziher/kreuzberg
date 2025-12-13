import { extractBytes, initWasm } from "@kreuzberg/wasm";
import type { ExtractionConfig } from "@kreuzberg/wasm";

async function extractWithChunkMetadata() {
  await initWasm();

  const bytes = new Uint8Array(await fetch("document.pdf").then((r) => r.arrayBuffer()));

  const config: ExtractionConfig = {
    chunking: {
      maxChars: 500,
      chunkOverlap: 50,
    },
  };

  const result = await extractBytes(bytes, "application/pdf", config);

  console.log("Document Metadata:", result.metadata);

  if (result.chunks) {
    result.chunks.forEach((chunk) => {
      console.log("Chunk Metadata:", {
        charStart: chunk.metadata.charStart,
        charEnd: chunk.metadata.charEnd,
        index: chunk.metadata.chunkIndex,
        total: chunk.metadata.totalChunks,
        tokens: chunk.metadata.tokenCount,
      });
    });
  }
}

extractWithChunkMetadata().catch(console.error);
