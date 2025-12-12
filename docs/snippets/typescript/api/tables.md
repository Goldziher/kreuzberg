```typescript title="TypeScript"
import { extractFileSync } from '@kreuzberg/node';

const result = extractFileSync('document.pdf');

for (const table of result.tables) {
	console.log(`Table with ${table.cells.length} rows`);
	console.log(`Page: ${table.pageNumber}`);
	console.log(table.markdown);
}
```
