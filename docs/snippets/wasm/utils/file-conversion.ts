import { fileToUint8Array, extractBytes, initWasm } from "@kreuzberg/wasm";

async function convertAndExtract(file: File) {
  await initWasm();

  try {
    if (file.size > 512 * 1024 * 1024) {
      throw new Error("File exceeds 512 MB limit");
    }

    const bytes = await fileToUint8Array(file);
    console.log(`Converted ${file.name} (${bytes.byteLength} bytes) to Uint8Array`);

    const result = await extractBytes(bytes, file.type);
    return result;
  } catch (error) {
    console.error("Conversion failed:", error);
    throw error;
  }
}

function createBlobFromResult(result: any): Blob {
  const json = JSON.stringify({
    content: result.content,
    mimeType: result.mimeType,
    metadata: result.metadata,
  });

  return new Blob([json], { type: "application/json" });
}

async function demonstrateConversion() {
  const file = new File([new ArrayBuffer(100)], "test.pdf", { type: "application/pdf" });

  try {
    const result = await convertAndExtract(file);
    const blob = createBlobFromResult(result);
    console.log("Result blob:", blob);
  } catch (error) {
    console.error(error);
  }
}

demonstrateConversion();
