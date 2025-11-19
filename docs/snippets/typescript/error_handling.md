```typescript
import {
    extractFileSync,
    ExtractionConfig,
    KreuzbergError
} from 'kreuzberg';

try {
    const result = extractFileSync('document.pdf', null, new ExtractionConfig());
    console.log(result.content);
} catch (error) {
    if (error instanceof KreuzbergError) {
        console.error(`Extraction error: ${error.message}`);
    } else {
        throw error;
    }
}
```
