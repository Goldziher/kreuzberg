import { extractFromFile, initWasm } from "@kreuzberg/wasm";

async function setupFileInput() {
  await initWasm();

  const fileInput = document.getElementById("file-input") as HTMLInputElement;

  fileInput.addEventListener("change", async (event) => {
    const file = (event.target as HTMLInputElement).files?.[0];
    if (!file) return;

    try {
      const result = await extractFromFile(file);
      console.log("Extracted text:", result.content);
      displayResults(result);
    } catch (error) {
      console.error("Extraction failed:", error);
    }
  });
}

function displayResults(result: any) {
  const output = document.getElementById("output");
  if (output) {
    output.textContent = result.content.substring(0, 500) + "...";
  }
}

setupFileInput().catch(console.error);
