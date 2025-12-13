import { extractBytes, initWasm } from "@kreuzberg/wasm";

interface DocumentJob {
  name: string;
  bytes: Uint8Array;
  mimeType: string;
}

async function processBatch(documents: DocumentJob[], concurrency: number = 3) {
  await initWasm();

  const results: Record<string, string> = {};
  const queue = [...documents];

  const workers = Array(concurrency)
    .fill(null)
    .map(async () => {
      while (queue.length > 0) {
        const doc = queue.shift();
        if (!doc) break;

        try {
          const result = await extractBytes(doc.bytes, doc.mimeType);
          results[doc.name] = result.content;
        } catch (error) {
          console.error(`Failed to process ${doc.name}:`, error);
        }
      }
    });

  await Promise.all(workers);
  return results;
}
