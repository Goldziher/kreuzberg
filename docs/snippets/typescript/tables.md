```typescript
import { extractFileSync, ExtractionConfig } from "kreuzberg";

const result = extractFileSync("document.pdf", null, new ExtractionConfig());

// Iterate over tables
for (const table of result.tables) {
	console.log(`Table with ${table.cells.length} rows`);
	console.log(table.markdown); // Markdown representation

	// Access cells
	for (const row of table.cells) {
		console.log(row);
	}
}
```
