```typescript
import {
	extractFile,
	type ExtractionConfig,
	type ExtractionResult,
} from 'kreuzberg';

/**
 * Note: Custom cache backends are not supported in TypeScript v4.0.
 * Caching is handled internally by the Rust core.
 *
 * This example demonstrates the config structure.
 * To enable caching, use the useCache flag.
 */

// Usage with built-in cache
const config: ExtractionConfig = {
  useCache: true,  // Enable internal Rust cache
};

const result = await extractFile("document.pdf", null, config);
```
