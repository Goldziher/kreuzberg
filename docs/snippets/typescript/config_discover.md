```typescript
import { extractFile, ExtractionConfig } from 'kreuzberg';

const config = ExtractionConfig.discover();
const result = await extractFile('document.pdf', null, config);
console.log(result.content);
```
