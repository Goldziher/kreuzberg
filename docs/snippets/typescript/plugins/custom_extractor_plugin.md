```typescript
import {
  listDocumentExtractors,
  unregisterDocumentExtractor,
  clearDocumentExtractors,
} from "@goldziher/kreuzberg";

/**
 * Note: Custom document extractors are not supported in TypeScript v4.0.
 * Document extraction logic lives in the Rust core.
 *
 * You can list, unregister, or clear built-in extractors.
 */

// List all registered document extractors
const extractors = listDocumentExtractors();
console.log("Available extractors:", extractors);

// Unregister a specific extractor (use with caution)
unregisterDocumentExtractor("SomeExtractor");

// Clear all extractors (use with extreme caution)
// clearDocumentExtractors();
```
