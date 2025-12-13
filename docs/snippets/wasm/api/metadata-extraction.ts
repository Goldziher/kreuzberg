import { extractBytes, initWasm } from "@kreuzberg/wasm";

async function extractMetadata() {
  await initWasm();

  const bytes = new Uint8Array(await fetch("document.pdf").then((r) => r.arrayBuffer()));

  const result = await extractBytes(bytes, "application/pdf");

  console.log("Document Metadata:");
  console.log("==================");
  console.log("MIME Type:", result.mimeType);
  console.log("Metadata:", result.metadata);

  if (result.detectedLanguages) {
    console.log("Detected Languages:", result.detectedLanguages.join(", "));
  }

  if (result.metadata.pageCount) {
    console.log("Page Count:", result.metadata.pageCount);
  }

  if (result.metadata.author) {
    console.log("Author:", result.metadata.author);
  }

  if (result.metadata.createdAt) {
    console.log("Created:", new Date(result.metadata.createdAt).toISOString());
  }
}

extractMetadata().catch(console.error);
