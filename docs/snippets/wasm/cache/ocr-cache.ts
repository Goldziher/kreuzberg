import { enableOcr, extractBytes, initWasm, TesseractWasmBackend } from "@kreuzberg/wasm";

async function demonstrateOcrCaching() {
  await initWasm();

  const backend = new TesseractWasmBackend();
  await backend.initialize();

  console.log("Tesseract WASM backend loaded - models cached");

  const imageBytes = new Uint8Array(await fetch("page1.png").then((r) => r.arrayBuffer()));

  console.time("First OCR (with model load)");
  const result1 = await extractBytes(imageBytes, "image/png", {
    ocr: { backend: "tesseract-wasm", language: "eng" },
  });
  console.timeEnd("First OCR (with model load)");

  console.log("Model cached in memory");

  const imageBytes2 = new Uint8Array(await fetch("page2.png").then((r) => r.arrayBuffer()));

  console.time("Second OCR (model cached)");
  const result2 = await extractBytes(imageBytes2, "image/png", {
    ocr: { backend: "tesseract-wasm", language: "eng" },
  });
  console.timeEnd("Second OCR (model cached)");
}

demonstrateOcrCaching().catch(console.error);
