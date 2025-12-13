import { extractBytes, initWasm } from "@kreuzberg/wasm";

async function extractStreamingDocument(url: string) {
  await initWasm();

  const response = await fetch(url);
  if (!response.ok) throw new Error(`HTTP ${response.status}`);

  const reader = response.body?.getReader();
  if (!reader) throw new Error("No response body");

  const chunks: Uint8Array[] = [];
  let totalSize = 0;

  while (true) {
    const { done, value } = await reader.read();
    if (done) break;

    chunks.push(value);
    totalSize += value.length;
    console.log(`Received ${totalSize} bytes`);
  }

  const fullBuffer = new Uint8Array(totalSize);
  let offset = 0;
  for (const chunk of chunks) {
    fullBuffer.set(chunk, offset);
    offset += chunk.length;
  }

  console.log("Document fully received, extracting...");
  const result = await extractBytes(fullBuffer, "application/pdf");

  return result;
}

extractStreamingDocument("https://example.com/document.pdf")
  .then((r) => console.log(r))
  .catch(console.error);
