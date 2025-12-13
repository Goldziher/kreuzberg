import { extractBytes, initWasm } from "@kreuzberg/wasm";

async function getDocumentMetadata() {
  await initWasm();

  const bytes = new Uint8Array(await fetch("document.pdf").then((r) => r.arrayBuffer()));

  const result = await extractBytes(bytes, "application/pdf");

  const metadata = result.metadata;

  console.log("Document Metadata:");
  console.log("Title:", metadata.title);
  console.log("Author:", metadata.author);
  console.log("Creator:", metadata.creator);
  console.log("Subject:", metadata.subject);
  console.log("Keywords:", metadata.keywords);
  console.log("Pages:", metadata.pageCount);
  console.log("Created:", metadata.createdAt);
  console.log("Modified:", metadata.modifiedAt);

  return metadata;
}

getDocumentMetadata().catch(console.error);
