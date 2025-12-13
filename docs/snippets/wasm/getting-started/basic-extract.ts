import { extractBytes, initWasm } from "@kreuzberg/wasm";

async function main() {
  await initWasm();

  const buffer = await fetch("document.pdf").then((r) => r.arrayBuffer());
  const bytes = new Uint8Array(buffer);

  const result = await extractBytes(bytes, "application/pdf");

  console.log("Extracted content:");
  console.log(result.content);
  console.log("MIME type:", result.mimeType);
  console.log("Metadata:", result.metadata);
}

main().catch(console.error);
