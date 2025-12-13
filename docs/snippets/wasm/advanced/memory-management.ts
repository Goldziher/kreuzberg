import { extractBytes, initWasm } from "@kreuzberg/wasm";

async function processLargeDocumentSet(files: File[]) {
  await initWasm();

  const BATCH_SIZE = 5;
  const results: any[] = [];

  for (let i = 0; i < files.length; i += BATCH_SIZE) {
    const batch = files.slice(i, i + BATCH_SIZE);

    console.log(`Processing batch ${Math.floor(i / BATCH_SIZE) + 1}`);

    const batchResults = await Promise.all(
      batch.map(async (file) => {
        const arrayBuffer = await file.arrayBuffer();
        const bytes = new Uint8Array(arrayBuffer);
        return extractBytes(bytes, file.type);
      })
    );

    results.push(...batchResults);

    if (global.gc) {
      console.log("Running garbage collection");
      global.gc();
    }

    await new Promise((resolve) => setTimeout(resolve, 100));
  }

  return results;
}

processLargeDocumentSet([]).then(() => console.log("Done")).catch(console.error);
