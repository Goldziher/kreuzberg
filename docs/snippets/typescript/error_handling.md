```typescript
import { extractFileSync, KreuzbergError } from 'kreuzberg';

try {
	const result = extractFileSync('document.pdf');
	console.log(result.content);
} catch (error) {
	if (error instanceof KreuzbergError) {
		console.error(`Extraction error: ${error.message}`);
	} else {
		throw error;
	}
}
```
