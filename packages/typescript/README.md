# Kreuzberg for TypeScript/Node.js

[![npm version](https://img.shields.io/npm/v/@goldziher/kreuzberg)](https://www.npmjs.com/package/@goldziher/kreuzberg)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

High-performance document intelligence for TypeScript and Node.js. Extract text, metadata, and structured information from PDFs, Office documents, images, and 50+ formats.

**Powered by a Rust core** – 10-50x faster than pure-JavaScript alternatives.

## Installation

```bash
npm install @goldziher/kreuzberg
```

```bash
pnpm add @goldziher/kreuzberg
```

```bash
yarn add @goldziher/kreuzberg
```

## Quick Start

### Simple Extraction

```typescript
import { extractFileSync } from '@goldziher/kreuzberg';

const result = extractFileSync('document.pdf');
console.log(result.content);
```

### Async Extraction (Recommended)

```typescript
import { extractFile } from '@goldziher/kreuzberg';

const result = await extractFile('document.pdf');
console.log(result.content);
```

### Batch Processing (Recommended for Multiple Files)

```typescript
import { batchExtractFiles } from '@goldziher/kreuzberg';

const files = ['doc1.pdf', 'doc2.docx', 'doc3.xlsx'];
const results = await batchExtractFiles(files);

for (const result of results) {
  console.log(result.content);
}
```

## OCR Support

### With Tesseract

```typescript
import { extractFileSync, type ExtractionConfig } from '@goldziher/kreuzberg';

const config: ExtractionConfig = {
  ocr: {
    backend: 'tesseract',
    language: 'eng',
  },
};

const result = extractFileSync('scanned.pdf', null, config);
```

### Table Extraction

```typescript
import { extractFileSync, type ExtractionConfig } from '@goldziher/kreuzberg';

const config: ExtractionConfig = {
  ocr: {
    backend: 'tesseract',
    tesseractConfig: {
      enableTableDetection: true,
    },
  },
};

const result = extractFileSync('invoice.pdf', null, config);

for (const table of result.tables) {
  console.log(table.markdown);
  console.log(table.cells);
}
```

## Configuration

### Complete Configuration Example

```typescript
import { extractFileSync, type ExtractionConfig } from '@goldziher/kreuzberg';

const config: ExtractionConfig = {
  useCache: true,
  enableQualityProcessing: true,
  ocr: {
    backend: 'tesseract',
    language: 'eng',
    tesseractConfig: {
      psm: 6,
      enableTableDetection: true,
      minConfidence: 50.0,
    },
  },
  forceOcr: false,
  chunking: {
    maxChars: 1000,
    maxOverlap: 200,
  },
  images: {
    extractImages: true,
    targetDpi: 300,
    maxImageDimension: 4096,
    autoAdjustDpi: true,
  },
  pdfOptions: {
    extractImages: true,
    passwords: ['password1', 'password2'],
    extractMetadata: true,
  },
  tokenReduction: {
    mode: 'moderate',
    preserveImportantWords: true,
  },
  languageDetection: {
    enabled: true,
    minConfidence: 0.8,
    detectMultiple: false,
  },
};

const result = extractFileSync('document.pdf', null, config);
```

## Metadata Extraction

```typescript
import { extractFileSync } from '@goldziher/kreuzberg';

const result = extractFileSync('document.pdf');

console.log(result.metadata.pdf?.title);
console.log(result.metadata.pdf?.author);
console.log(result.metadata.pdf?.pageCount);
console.log(result.metadata.pdf?.creationDate);
console.log(result.metadata.language);
console.log(result.metadata.format);
```

## Password-Protected PDFs

```typescript
import { extractFileSync, type ExtractionConfig } from '@goldziher/kreuzberg';

const config: ExtractionConfig = {
  pdfOptions: {
    passwords: ['password1', 'password2', 'password3'],
  },
};

const result = extractFileSync('protected.pdf', null, config);
```

## Language Detection

```typescript
import { extractFileSync, type ExtractionConfig } from '@goldziher/kreuzberg';

const config: ExtractionConfig = {
  languageDetection: {
    enabled: true,
  },
};

const result = extractFileSync('multilingual.pdf', null, config);
console.log(result.detectedLanguages);
```

## Text Chunking

```typescript
import { extractFileSync, type ExtractionConfig } from '@goldziher/kreuzberg';

const config: ExtractionConfig = {
  chunking: {
    maxChars: 1000,
    maxOverlap: 200,
  },
};

const result = extractFileSync('long_document.pdf', null, config);

for (const chunk of result.chunks || []) {
  console.log(chunk);
}
```

## Extract from Bytes

```typescript
import { extractBytesSync } from '@goldziher/kreuzberg';
import { readFileSync } from 'fs';

const data = readFileSync('document.pdf');
const result = extractBytesSync(data, 'application/pdf');
console.log(result.content);
```

## API Reference

### Extraction Functions

- `extractFile(filePath, mimeType?, config?)` – Async extraction
- `extractFileSync(filePath, mimeType?, config?)` – Sync extraction
- `extractBytes(data, mimeType, config?)` – Async extraction from bytes
- `extractBytesSync(data, mimeType, config?)` – Sync extraction from bytes
- `batchExtractFiles(paths, config?)` – Async batch extraction
- `batchExtractFilesSync(paths, config?)` – Sync batch extraction
- `batchExtractBytes(dataList, mimeTypes, config?)` – Async batch from bytes
- `batchExtractBytesSync(dataList, mimeTypes, config?)` – Sync batch from bytes

### Plugin Functions

- `registerPostProcessor(processor)` – Register custom post-processor
- `unregisterPostProcessor(name)` – Unregister post-processor
- `clearPostProcessors()` – Clear all post-processors
- `registerValidator(validator)` – Register custom validator
- `unregisterValidator(name)` – Unregister validator
- `clearValidators()` – Clear all validators

### Types

- `ExtractionConfig` – Main configuration
- `ExtractionResult` – Result with `content`, `metadata`, `tables`, `detectedLanguages`, `chunks`
- `Table` – Table with `cells`, `markdown`, `pageNumber`
- `Metadata` – Typed metadata object
- `PostProcessorProtocol` – Protocol for custom processors
- `ValidatorProtocol` – Protocol for custom validators

## Examples

### Custom Post-Processor

```typescript
import { registerPostProcessor, extractFile, type PostProcessorProtocol, type ExtractionResult } from '@goldziher/kreuzberg';

class MyProcessor implements PostProcessorProtocol {
  name(): string {
    return 'my_processor';
  }

  process(result: ExtractionResult): ExtractionResult {
    result.metadata.customField = 'custom_value';
    return result;
  }

  processingStage(): 'early' | 'middle' | 'late' {
    return 'middle';
  }
}

registerPostProcessor(new MyProcessor());

const result = await extractFile('document.pdf');
console.log(result.metadata.customField);
```

### Multiple Files with Progress

```typescript
import { batchExtractFiles } from '@goldziher/kreuzberg';
import { glob } from 'glob';

const files = await glob('documents/*.pdf');
console.log(`Processing ${files.length} files...`);

const results = await batchExtractFiles(files);

for (let i = 0; i < results.length; i++) {
  console.log(`${files[i]}: ${results[i].content.length} characters`);
}
```

### Filter by Language

```typescript
import { extractFileSync, type ExtractionConfig } from '@goldziher/kreuzberg';

const config: ExtractionConfig = {
  languageDetection: {
    enabled: true,
  },
};

const result = extractFileSync('document.pdf', null, config);

if (result.detectedLanguages?.includes('en')) {
  console.log('English document detected');
  console.log(result.content);
}
```

## System Requirements

### Node.js

- Node.js 18.x or higher
- Supports both CommonJS and ES modules

### System Dependencies

**Tesseract OCR** (Required for OCR):
```bash
brew install tesseract
```

**LibreOffice** (Optional, for .doc and .ppt):
```bash
brew install libreoffice
```

**Pandoc** (Optional, for some formats):
```bash
brew install pandoc
```

## Troubleshooting

### Module not found: kreuzberg-node

The native binding wasn't installed correctly. Try:

```bash
npm install --force @goldziher/kreuzberg
```

### Type errors with TypeScript

Make sure you're using TypeScript 4.5 or higher:

```bash
npm install --save-dev typescript@latest
```

### OCR not working

Make sure Tesseract is installed and in your PATH:

```bash
tesseract --version
```

## Complete Documentation

**[https://kreuzberg.dev](https://kreuzberg.dev)**

- [Installation Guide](https://kreuzberg.dev/getting-started/installation/)
- [User Guide](https://kreuzberg.dev/user-guide/)
- [API Reference](https://kreuzberg.dev/api-reference/)
- [Format Support](https://kreuzberg.dev/formats/)
- [OCR Backends](https://kreuzberg.dev/user-guide/ocr-backends/)

## License

MIT License - see [LICENSE](../../LICENSE) for details.
