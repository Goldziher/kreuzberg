import { extractFromFile, initWasm } from "@kreuzberg/wasm";

async function handleBrowserFileInput() {
  await initWasm();

  const input = document.getElementById("file-upload") as HTMLInputElement;

  input.addEventListener("change", async (event) => {
    const file = (event.target as HTMLInputElement).files?.[0];
    if (!file) return;

    try {
      const result = await extractFromFile(file);

      const output = document.getElementById("output");
      if (output) {
        output.innerHTML = `
          <h3>Extraction Result</h3>
          <p><strong>MIME Type:</strong> ${result.mimeType}</p>
          <p><strong>Content Preview:</strong></p>
          <pre>${result.content.substring(0, 500)}</pre>
        `;
      }
    } catch (error) {
      console.error("Extraction failed:", error);
    }
  });
}

handleBrowserFileInput().catch(console.error);
