```typescript
import { extractFile, type ExtractionConfig } from "@kreuzberg/node";

const config: ExtractionConfig = { useCache: true };

(async () => {
  console.log("First extraction (will be cached)...");
  const result1 = await extractFile("document.pdf", null, config);
  console.log(`  - Content length: ${result1.content.length}`);

  console.log("\nSecond extraction (from cache)...");
  const result2 = await extractFile("document.pdf", null, config);
  console.log(`  - Content length: ${result2.content.length}`);

  console.log(`\nResults are identical: ${result1.content === result2.content}`);
})();
```
