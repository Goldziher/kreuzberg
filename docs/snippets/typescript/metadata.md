```typescript
import { extractFileSync, ExtractionConfig } from "kreuzberg";

const result = extractFileSync("document.pdf", null, new ExtractionConfig());

// Access PDF metadata
if (result.metadata.pdf) {
	console.log(`Pages: ${result.metadata.pdf.pageCount}`);
	console.log(`Author: ${result.metadata.pdf.author}`);
	console.log(`Title: ${result.metadata.pdf.title}`);
}

// Access HTML metadata
const htmlResult = extractFileSync("page.html", null, new ExtractionConfig());
if (htmlResult.metadata.html) {
	console.log(`Title: ${htmlResult.metadata.html.title}`);
	console.log(`Description: ${htmlResult.metadata.html.description}`);
	console.log(`Open Graph Image: ${htmlResult.metadata.html.ogImage}`);
}
```
