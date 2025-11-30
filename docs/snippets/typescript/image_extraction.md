```typescript
import { extractFile } from '@kreuzberg/node';

const config = {
	images: {
		extractImages: true,
		targetDpi: 200,
		maxImageDimension: 2048,
		autoAdjustDpi: true,
	},
};

const result = await extractFile('document.pdf', null, config);
console.log(`Extracted ${result.images?.length ?? 0} images`);
```
