import {
  TesseractWasmBackend,
  registerOcrBackend,
  unregisterOcrBackend,
  listOcrBackends,
  initWasm,
} from "@kreuzberg/wasm";

async function manageOcrBackends() {
  await initWasm();

  const backend = new TesseractWasmBackend();
  await backend.initialize();

  registerOcrBackend(backend);

  const backends = listOcrBackends();
  console.log("Available OCR backends:", backends);

  if (backends.includes("tesseract-wasm")) {
    console.log("Tesseract WASM backend is registered");
  }

  unregisterOcrBackend("tesseract-wasm");

  const afterUnregister = listOcrBackends();
  console.log("Backends after unregister:", afterUnregister);
}

manageOcrBackends().catch(console.error);
