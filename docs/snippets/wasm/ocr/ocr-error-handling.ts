import { enableOcr, extractBytes, initWasm } from "@kreuzberg/wasm";

async function extractWithErrorHandling() {
  try {
    await initWasm();
  } catch (error) {
    console.error("Failed to initialize WASM:", error);
    return;
  }

  try {
    await enableOcr();
  } catch (error) {
    if (error instanceof Error && error.message.includes("browser")) {
      console.warn("OCR not available in this environment, proceeding without OCR");
    } else {
      throw error;
    }
  }

  try {
    const bytes = new Uint8Array(await fetch("document.png").then((r) => r.arrayBuffer()));

    const result = await extractBytes(bytes, "image/png", {
      ocr: {
        backend: "tesseract-wasm",
        language: "eng",
      },
    });

    console.log("Extraction successful:", result.content.length, "chars");
  } catch (error) {
    console.error("Extraction failed:", error);
  }
}

extractWithErrorHandling().catch(console.error);
