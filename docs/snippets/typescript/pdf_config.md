```typescript
import { extractFile } from 'kreuzberg';

const config = {
	pdfOptions: {
		extractImages: true,
		extractMetadata: true,
		passwords: ['password1', 'password2'],
	},
};

const result = await extractFile('document.pdf', null, config);
console.log(result.content);
```
