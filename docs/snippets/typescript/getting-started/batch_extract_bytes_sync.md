```typescript title="TypeScript"
import { batchExtractBytesSync } from '@kreuzberg/node';
import { readFileSync } from 'fs';

const files = ['doc1.pdf', 'doc2.docx'];
const dataList = files.map(f => readFileSync(f));
const mimeTypes = [
	'application/pdf',
	'application/vnd.openxmlformats-officedocument.wordprocessingml.document'
];

const results = batchExtractBytesSync(dataList, mimeTypes);

results.forEach((result, i) => {
	console.log(`Document ${i + 1}: ${result.content.length} characters`);
});
```
