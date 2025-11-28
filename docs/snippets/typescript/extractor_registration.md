```typescript
import { extractFileSync, listDocumentExtractors } from 'kreuzberg';

const extractors = listDocumentExtractors();
console.log('Available extractors:', extractors);

const result = extractFileSync('document.json');
console.log(`Extracted content length: ${result.content.length}`);
```
