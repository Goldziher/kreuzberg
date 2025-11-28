```typescript
import { extractBytes } from 'kreuzberg';
import { readFile } from 'fs/promises';

const data = await readFile('document.pdf');
const result = await extractBytes(data, 'application/pdf');
console.log(result.content);
```
