```typescript
import {
  listDocumentExtractors,
  unregisterDocumentExtractor,
} from "@goldziher/kreuzberg";

/**
 * Note: Custom document extractors are not directly supported in TypeScript v4.0.
 * Document extraction logic lives in the Rust core.
 *
 * You can list and unregister built-in extractors, but cannot add custom ones
 * from TypeScript. For custom extractors, implement them in Rust.
 */

// List all registered document extractors
const extractors = listDocumentExtractors();
console.log("Available extractors:", extractors);
// Example output: ['PDFExtractor', 'ImageExtractor', 'OfficeExtractor', ...]

// Unregister a built-in extractor (use with caution)
unregisterDocumentExtractor("SomeExtractor");
```
