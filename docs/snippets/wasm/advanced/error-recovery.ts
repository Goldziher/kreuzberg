import { extractBytes, initWasm } from "@kreuzberg/wasm";

async function extractWithRetry(
  bytes: Uint8Array,
  mimeType: string,
  maxRetries: number = 3
) {
  await initWasm();

  let lastError: Error | null = null;

  for (let attempt = 1; attempt <= maxRetries; attempt++) {
    try {
      console.log(`Extraction attempt ${attempt}/${maxRetries}`);
      const result = await extractBytes(bytes, mimeType);
      console.log("Extraction successful");
      return result;
    } catch (error) {
      lastError = error instanceof Error ? error : new Error(String(error));
      console.warn(`Attempt ${attempt} failed:`, lastError.message);

      if (attempt < maxRetries) {
        const delay = Math.pow(2, attempt) * 100;
        console.log(`Retrying in ${delay}ms...`);
        await new Promise((resolve) => setTimeout(resolve, delay));
      }
    }
  }

  throw new Error(`Extraction failed after ${maxRetries} attempts: ${lastError?.message}`);
}

extractWithRetry(new Uint8Array(await fetch("doc.pdf").then((r) => r.arrayBuffer())), "application/pdf")
  .then((r) => console.log("Final result:", r))
  .catch(console.error);
