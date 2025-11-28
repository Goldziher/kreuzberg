```typescript
import { batchExtractFilesSync } from 'kreuzberg';

const files = ['doc1.pdf', 'doc2.docx', 'doc3.pptx'];
const results = batchExtractFilesSync(files);

results.forEach((result, i) => {
	console.log(`File ${i + 1}: ${result.content.length} characters`);
});
```
