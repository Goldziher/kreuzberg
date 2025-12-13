import { extractBytes, initWasm } from "@kreuzberg/wasm";

async function extractPdfDocument() {
  await initWasm();

  const response = await fetch("sample.pdf");
  const arrayBuffer = await response.arrayBuffer();
  const bytes = new Uint8Array(arrayBuffer);

  const result = await extractBytes(bytes, "application/pdf");

  console.log("Text content:", result.content);
  console.log("Detected languages:", result.detectedLanguages);
  console.log("Page count:", result.metadata.pageCount);

  if (result.tables && result.tables.length > 0) {
    console.log("Found tables:", result.tables.length);
  }
}

extractPdfDocument().catch(console.error);
