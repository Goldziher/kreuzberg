```typescript
import { extractFileSync } from '@kreuzberg/node';

const result = extractFileSync('document.pdf');
console.log(`Metadata: ${JSON.stringify(result.metadata)}`);
if (result.metadata.page_count) {
	console.log(`Pages: ${result.metadata.page_count}`);
}

const htmlResult = extractFileSync('page.html');
console.log(`HTML Metadata: ${JSON.stringify(htmlResult.metadata)}`);
if (htmlResult.metadata.title) {
	console.log(`Title: ${htmlResult.metadata.title}`);
}
```
