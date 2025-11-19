```typescript
import { extractBytes, ExtractionConfig } from 'kreuzberg';
import { readFile } from 'fs/promises';

async function main() {
    const data = await readFile('document.pdf');

    const result = await extractBytes(
        data,
        'application/pdf',
        new ExtractionConfig()
    );
    console.log(result.content);
}

main();
```
