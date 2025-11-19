```typescript
import { extractBytesSync, ExtractionConfig } from 'kreuzberg';
import { readFileSync } from 'fs';

const data = readFileSync('document.pdf');

const result = extractBytesSync(
    data,
    'application/pdf',
    new ExtractionConfig()
);
console.log(result.content);
```
