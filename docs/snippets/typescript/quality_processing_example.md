```typescript
import { extractFile, ExtractionConfig } from '@kreuzberg/sdk';

const config = new ExtractionConfig({ enableQualityProcessing: true });
const result = await extractFile('scanned_document.pdf', { config });

const qualityScore = result.metadata.quality_score || 0.0;

if (qualityScore < 0.5) {
  console.log(`Warning: Low quality extraction (${qualityScore.toFixed(2)})`);
  console.log('Consider re-scanning with higher DPI or adjusting OCR settings');
} else {
  console.log(`Quality score: ${qualityScore.toFixed(2)}`);
}
```
