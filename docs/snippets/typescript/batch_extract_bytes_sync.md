```typescript
import { batchExtractBytesSync, ExtractionConfig } from 'kreuzberg';
import { readFileSync } from 'fs';

const files = ['doc1.pdf', 'doc2.docx'];
const dataList = files.map(f => readFileSync(f));
const mimeTypes = files.map(f =>
    f.endsWith('.pdf') ? 'application/pdf' :
    'application/vnd.openxmlformats-officedocument.wordprocessingml.document'
);

const results = batchExtractBytesSync(
    dataList,
    mimeTypes,
    new ExtractionConfig()
);

results.forEach((result, i) => {
    console.log(`Document ${i + 1}: ${result.content.length} characters`);
});
```
