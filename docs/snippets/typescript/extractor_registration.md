```typescript
import { registerDocumentExtractor, extractFileSync } from "kreuzberg";

// Register custom extractor with priority 50
registerDocumentExtractor("custom-json-extractor", async (content, mimeType, config) => {
    const parsed = JSON.parse(content);
    return {
        content: JSON.stringify(parsed),
        mime_type: "text/plain",
        metadata: {},
        tables: [],
    };
}, 50);

const result = extractFileSync("document.json");
console.log(`Extracted content length: ${result.content.length}`);
```
