```typescript
import { extractFile, ExtractionConfig } from 'kreuzberg';

async function main() {
    const result = await extractFile('document.pdf', null, new ExtractionConfig());
    console.log(result.content);
}

main();
```
