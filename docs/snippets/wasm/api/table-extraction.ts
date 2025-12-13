import { extractBytes, initWasm } from "@kreuzberg/wasm";

async function extractTables() {
  await initWasm();

  const bytes = new Uint8Array(await fetch("spreadsheet.pdf").then((r) => r.arrayBuffer()));

  const result = await extractBytes(bytes, "application/pdf");

  if (result.tables && result.tables.length > 0) {
    result.tables.forEach((table, index) => {
      console.log(`\nTable ${index} (Page ${table.pageNumber}):`);
      console.log("Markdown representation:");
      console.log(table.markdown);

      console.log("Cell data:");
      table.cells.forEach((row, rowIndex) => {
        console.log(`  Row ${rowIndex}:`, row);
      });
    });
  } else {
    console.log("No tables found in document");
  }
}

extractTables().catch(console.error);
