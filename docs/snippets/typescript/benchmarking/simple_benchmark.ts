```typescript
import { extractFile, type ExtractionConfig } from "@kreuzberg/node";

async function benchmarkExtractions(): Promise<void> {
  const config: ExtractionConfig = { useCache: false };
  const filePath = "document.pdf";
  const numRuns = 10;

  let start = performance.now();
  for (let i = 0; i < numRuns; i++) {
    await extractFile(filePath, null, config);
  }
  const syncDuration = (performance.now() - start) / 1000;
  const avgSync = syncDuration / numRuns;

  console.log(`Sync extraction (${numRuns} runs):`);
  console.log(`  - Total time: ${syncDuration.toFixed(3)}s`);
  console.log(`  - Average: ${avgSync.toFixed(3)}s per extraction`);

  start = performance.now();
  const tasks = Array(numRuns)
    .fill(null)
    .map(() => extractFile(filePath, null, config));
  await Promise.all(tasks);
  const asyncDuration = (performance.now() - start) / 1000;

  console.log(`\nAsync extraction (${numRuns} parallel runs):`);
  console.log(`  - Total time: ${asyncDuration.toFixed(3)}s`);
  console.log(`  - Average: ${(asyncDuration / numRuns).toFixed(3)}s per extraction`);
  console.log(`  - Speedup: ${(syncDuration / asyncDuration).toFixed(1)}x`);

  const cacheConfig: ExtractionConfig = { useCache: true };

  console.log("\nFirst extraction (populates cache)...");
  start = performance.now();
  const result1 = await extractFile(filePath, null, cacheConfig);
  const firstDuration = (performance.now() - start) / 1000;
  console.log(`  - Time: ${firstDuration.toFixed(3)}s`);

  console.log("Second extraction (from cache)...");
  start = performance.now();
  const result2 = await extractFile(filePath, null, cacheConfig);
  const cachedDuration = (performance.now() - start) / 1000;
  console.log(`  - Time: ${cachedDuration.toFixed(3)}s`);
  console.log(`  - Cache speedup: ${(firstDuration / cachedDuration).toFixed(1)}x`);
}

benchmarkExtractions().catch(console.error);
```
