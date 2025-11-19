```typescript
import { batchExtractFilesSync, ExtractionConfig } from 'kreuzberg';

const files = ['doc1.pdf', 'doc2.docx', 'doc3.pptx'];
const config = new ExtractionConfig();

const results = batchExtractFilesSync(files, config);

results.forEach((result, i) => {
    console.log(`File ${i + 1}: ${result.content.length} characters`);
});
```
