import {
  initWasm,
  isInitialized,
  getVersion,
  getWasmCapabilities,
} from "@kreuzberg/wasm";

async function initializeKreuzberg() {
  const caps = getWasmCapabilities();

  if (!caps.hasWasm) {
    console.error("WebAssembly not supported");
    return;
  }

  try {
    if (!isInitialized()) {
      await initWasm();
    }

    const version = getVersion();
    console.log(`Kreuzberg ${version} initialized successfully`);
    console.log("Workers available:", caps.hasWorkers);
    console.log("SharedArrayBuffer available:", caps.hasSharedArrayBuffer);
  } catch (error) {
    console.error("Initialization failed:", error);
  }
}

initializeKreuzberg();
