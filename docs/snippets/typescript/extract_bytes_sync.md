```typescript
import { extractBytesSync } from '@kreuzberg/node';
import { readFileSync } from 'fs';

const data = readFileSync('document.pdf');
const result = extractBytesSync(data, 'application/pdf');
console.log(result.content);
```
