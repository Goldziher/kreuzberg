import { extractFile, initWasm } from "@kreuzberg/wasm";

async function extractFromFilePath() {
  await initWasm();

  try {
    const result = await extractFile("./documents/report.docx");

    console.log("Extracted content:", result.content);
    console.log("MIME type:", result.mimeType);
    console.log("Word count:", result.content.split(/\s+/).length);
  } catch (error) {
    if (error instanceof Error) {
      console.error("Extraction error:", error.message);
    }
  }
}

async function extractWithMimeType() {
  await initWasm();

  const result = await extractFile(
    "./data/spreadsheet.xlsx",
    "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
  );

  console.log("Extracted spreadsheet:", result.content);
}

extractFromFilePath().catch(console.error);
