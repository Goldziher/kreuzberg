import { enableOcr, extractBytes, initWasm } from "@kreuzberg/wasm";

async function extractMultilingualDocument() {
  await initWasm();
  await enableOcr();

  const documents = [
    { name: "english.png", lang: "eng" },
    { name: "german.png", lang: "deu" },
    { name: "spanish.png", lang: "spa" },
  ];

  for (const doc of documents) {
    const bytes = new Uint8Array(await fetch(doc.name).then((r) => r.arrayBuffer()));

    const result = await extractBytes(bytes, "image/png", {
      ocr: {
        backend: "tesseract-wasm",
        language: doc.lang,
      },
    });

    console.log(`${doc.name} (${doc.lang}):`);
    console.log(result.content);
    console.log("---");
  }
}

extractMultilingualDocument().catch(console.error);
