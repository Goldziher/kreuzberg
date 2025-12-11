```typescript
import { Kreuzberg, ExtractionConfig, CacheConfig } from "kreuzberg";
import { homedir } from "os";
import { join } from "path";
import { mkdirSync } from "fs";

const cacheDir = join(homedir(), ".cache", "kreuzberg");
mkdirSync(cacheDir, { recursive: true });

const config: ExtractionConfig = {
  useCache: true,
  cacheConfig: {
    cachePath: cacheDir,
    maxCacheSize: 500 * 1024 * 1024,
    cacheTtlSeconds: 7 * 86400,
    enableCompression: true,
  },
};

const kreuzberg = new Kreuzberg(config);

(async () => {
  console.log("First extraction (will be cached)...");
  const result1 = await kreuzberg.extractFile("document.pdf");
  console.log(`  - Content length: ${result1.content.length}`);
  console.log(`  - Cached: ${result1.metadata.wasCached ?? false}`);

  console.log("\nSecond extraction (from cache)...");
  const result2 = await kreuzberg.extractFile("document.pdf");
  console.log(`  - Content length: ${result2.content.length}`);
  console.log(`  - Cached: ${result2.metadata.wasCached ?? false}`);

  console.log(`\nResults are identical: ${result1.content === result2.content}`);

  const cacheStats = await kreuzberg.getCacheStats();
  console.log(`\nCache Statistics:`);
  console.log(`  - Total entries: ${cacheStats.totalEntries ?? 0}`);
  console.log(`  - Cache size: ${((cacheStats.cacheSizeBytes ?? 0) / 1024 / 1024).toFixed(1)} MB`);
  console.log(`  - Hit rate: ${((cacheStats.hitRate ?? 0) * 100).toFixed(1)}%`);
})();
```
