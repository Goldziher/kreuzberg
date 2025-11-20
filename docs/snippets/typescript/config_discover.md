```typescript
import { ExtractionConfig, extractFile } from '@kreuzberg/sdk';

const config = await ExtractionConfig.discover();
const result = await extractFile('document.pdf', { config });
```
