```typescript
import {
  extractFile,
  ExtractionConfig,
  OcrConfig,
  TesseractConfig,
  ImagePreprocessingConfig,
  PdfConfig,
  ImageExtractionConfig,
  ChunkingConfig,
  EmbeddingConfig,
  EmbeddingModelType,
  TokenReductionConfig,
  LanguageDetectionConfig,
  PostProcessorConfig,
} from '@kreuzberg/sdk';

const config = new ExtractionConfig({
  useCache: true,
  enableQualityProcessing: true,
  forceOcr: false,
  ocr: new OcrConfig({
    backend: 'tesseract',
    language: 'eng+fra',
    tesseractConfig: new TesseractConfig({
      psm: 3,
      oem: 3,
      minConfidence: 0.8,
      preprocessing: new ImagePreprocessingConfig({
        targetDpi: 300,
        denoise: true,
        deskew: true,
        contrastEnhance: true,
      }),
      enableTableDetection: true,
    }),
  }),
  pdfOptions: new PdfConfig({
    extractImages: true,
    extractMetadata: true,
  }),
  images: new ImageExtractionConfig({
    extractImages: true,
    targetDpi: 150,
    maxImageDimension: 2048,
  }),
  chunking: new ChunkingConfig({
    maxChars: 1000,
    maxOverlap: 200,
    embedding: new EmbeddingConfig({
      model: EmbeddingModelType.preset('all-MiniLM-L6-v2'),
      batchSize: 32,
    }),
  }),
  tokenReduction: new TokenReductionConfig({
    mode: 'moderate',
    preserveImportantWords: true,
  }),
  languageDetection: new LanguageDetectionConfig({
    enabled: true,
    minConfidence: 0.8,
    detectMultiple: false,
  }),
  postprocessor: new PostProcessorConfig({
    enabled: true,
  }),
});

const result = await extractFile('document.pdf', { config });
console.log(`Extracted content length: ${result.content.length}`);
```
