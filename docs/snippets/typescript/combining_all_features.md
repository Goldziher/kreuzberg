```typescript
import {
  extractFile,
  ExtractionConfig,
  ChunkingConfig,
  EmbeddingConfig,
  LanguageDetectionConfig,
  TokenReductionConfig,
  KeywordConfig,
  KeywordAlgorithm
} from '@kreuzberg/sdk';

const config = new ExtractionConfig({
  enableQualityProcessing: true,

  languageDetection: new LanguageDetectionConfig({
    enabled: true,
    detectMultiple: true
  }),

  tokenReduction: new TokenReductionConfig({
    mode: 'moderate',
    preserveMarkdown: true
  }),

  chunking: new ChunkingConfig({
    maxChars: 512,
    maxOverlap: 50,
    embedding: new EmbeddingConfig({
      model: 'balanced',
      normalize: true
    })
  }),

  keywords: new KeywordConfig({
    algorithm: KeywordAlgorithm.YAKE,
    maxKeywords: 10
  })
});

const result = await extractFile('document.pdf', { config });

console.log(`Quality: ${result.metadata.quality_score.toFixed(2)}`);
console.log(`Languages: ${result.detectedLanguages}`);
console.log(`Keywords: ${result.metadata.keywords.map(k => k.text)}`);
console.log(`Chunks: ${result.chunks.length} with ${result.chunks[0].embedding.length} dimensions`);
```
