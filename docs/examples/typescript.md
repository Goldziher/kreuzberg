# TypeScript Examples

This page provides comprehensive examples of using Kreuzberg with TypeScript and Node.js. All example code is available in the [`examples/typescript/`](https://github.com/Goldziher/kreuzberg/tree/main/examples/typescript) directory.

## Installation

```bash
npm install @goldziher/kreuzberg

# Or with yarn
yarn add @goldziher/kreuzberg

# Or with pnpm
pnpm add @goldziher/kreuzberg
```

## Basic Extraction

The `basic.ts` example demonstrates fundamental extraction patterns including synchronous and asynchronous extraction, working with bytes, and accessing metadata.

### Simple Extraction

```typescript
--8<-- "examples/typescript/basic.ts:8:15"
```

### Extraction with Configuration

```typescript
--8<-- "examples/typescript/basic.ts:17:28"
```

### Async Extraction

```typescript
--8<-- "examples/typescript/basic.ts:30:33"
```

### Extract from Bytes

```typescript
--8<-- "examples/typescript/basic.ts:35:42"
```

### Accessing Metadata

```typescript
--8<-- "examples/typescript/basic.ts:44:57"
```

### Batch Processing

```typescript
--8<-- "examples/typescript/batch.ts:8:19"
```

### Parallel Batch Processing

For better performance with multiple files, use async batch processing:

```typescript
--8<-- "examples/typescript/batch.ts:21:33"
```

### Error Handling

```typescript
--8<-- "examples/typescript/batch.ts:35:52"
```

## OCR Extraction

The `ocr.ts` example shows how to extract text from scanned documents and images using Tesseract OCR.

### Basic OCR

```typescript
--8<-- "examples/typescript/ocr.ts:15:27"
```

### OCR with Custom Language

```typescript
--8<-- "examples/typescript/ocr.ts:29:39"
```

### Force OCR on Text PDFs

Sometimes you want to extract images and run OCR even if the PDF already has text:

```typescript
--8<-- "examples/typescript/ocr.ts:41:52"
```

### OCR from Images

```typescript
--8<-- "examples/typescript/ocr.ts:54:70"
```

### Table Extraction with OCR

```typescript
--8<-- "examples/typescript/ocr.ts:72:92"
```

### Custom PSM Mode

Page Segmentation Mode (PSM) controls how Tesseract analyzes the page layout:

```typescript
--8<-- "examples/typescript/ocr.ts:94:112"
```

## Custom PostProcessors

The `custom-postprocessor.ts` example shows how to create PostProcessor plugins for custom data transformation, enrichment, and validation.

### PII Redaction PostProcessor

```typescript
--8<-- "examples/typescript/custom-postprocessor.ts:17:79"
```

### Metadata Enrichment PostProcessor

```typescript
--8<-- "examples/typescript/custom-postprocessor.ts:84:111"
```

### Text Normalization PostProcessor

```typescript
--8<-- "examples/typescript/custom-postprocessor.ts:116:143"
```

### Keyword Extraction PostProcessor

```typescript
--8<-- "examples/typescript/custom-postprocessor.ts:148:185"
```

### External API PostProcessor

```typescript
--8<-- "examples/typescript/custom-postprocessor.ts:190:217"
```

### Registering and Using PostProcessors

```typescript
--8<-- "examples/typescript/custom-postprocessor.ts:220:243"
```

## Custom Validators

The `custom-validator.ts` example demonstrates how to implement Validator plugins for fail-fast validation.

### Min Length Validator

```typescript
--8<-- "examples/typescript/custom-validator.ts:17:39"
```

### Metadata Validator

```typescript
--8<-- "examples/typescript/custom-validator.ts:44:70"
```

### PDF Validator

```typescript
--8<-- "examples/typescript/custom-validator.ts:75:115"
```

### Language Validator

```typescript
--8<-- "examples/typescript/custom-validator.ts:120:145"
```

### Quality Validator

```typescript
--8<-- "examples/typescript/custom-validator.ts:150:185"
```

### Async External Validator

```typescript
--8<-- "examples/typescript/custom-validator.ts:190:239"
```

### Registering and Using Validators

```typescript
--8<-- "examples/typescript/custom-validator.ts:241:270"
```

## Configuration Options

### ExtractionConfig

The `ExtractionConfig` class controls extraction behavior:

```typescript
import {
    ExtractionConfig,
    OcrConfig,
    ChunkingConfig,
    TesseractConfig,
} from '@goldziher/kreuzberg';

const config = new ExtractionConfig({
    // Quality processing
    enableQualityProcessing: true,

    // Caching
    useCache: true,

    // OCR configuration
    ocr: new OcrConfig({
        backend: 'tesseract',
        language: 'eng',
        tesseractConfig: new TesseractConfig({
            psm: 6,
            oem: 3,
            enableTableDetection: true,
            dpi: 300,
        }),
    }),

    // Force OCR even for text-based PDFs
    forceOcr: false,

    // Chunking for large documents
    chunking: new ChunkingConfig({
        maxChars: 1000,
        maxOverlap: 100,
    }),
});
```

### OcrConfig

Configure OCR behavior:

```typescript
import { OcrConfig, TesseractConfig } from '@goldziher/kreuzberg';

const ocrConfig = new OcrConfig({
    backend: 'tesseract', // "tesseract" is currently the only supported backend
    language: 'eng',      // Language code
    tesseractConfig: new TesseractConfig({
        psm: 6,                      // Page segmentation mode
        oem: 3,                      // OCR Engine Mode
        enableTableDetection: true,
        dpi: 300,
    }),
});
```

### ChunkingConfig

Configure content chunking for large documents with optional embedding generation:

```typescript
import { ExtractionConfig } from '@goldziher/kreuzberg';

// Basic chunking without embeddings
const basicChunking: ExtractionConfig = {
    chunking: {
        maxChars: 1000,   // Maximum characters per chunk
        maxOverlap: 100,  // Overlap between chunks (must be < maxChars)
    },
};

// Chunking with embedding generation
const chunkingWithEmbeddings: ExtractionConfig = {
    chunking: {
        maxChars: 1000,
        maxOverlap: 200,  // Must be < maxChars
        embedding: {
            model: {
                modelType: 'preset',
                value: 'fast',  // 384-dimensional embeddings
            },
            normalize: true,
            batchSize: 32,
        },
    },
};
```

## Working with Results

### ExtractionResult

The `ExtractionResult` interface contains all extraction information:

```typescript
import { extractFile } from '@goldziher/kreuzberg';

const result = await extractFile('document.pdf');

// Extracted text content
console.log(result.content);

// MIME type
console.log(result.mimeType);

// Metadata (varies by document type)
if (result.metadata.pdf) {
    console.log(`Pages: ${result.metadata.pdf.pageCount}`);
    console.log(`Author: ${result.metadata.pdf.author}`);
    console.log(`Title: ${result.metadata.pdf.title}`);
}

// Extracted tables
for (const table of result.tables) {
    console.log(table.markdown);
}

// Detected languages (if language detection enabled)
if (result.detectedLanguages) {
    console.log(`Languages: ${result.detectedLanguages}`);
}

// Chunks (if chunking enabled)
if (result.chunks) {
    result.chunks.forEach((chunk, i) => {
        console.log(`Chunk ${i + 1}: ${chunk.content.slice(0, 50)}...`);

        // Access embedding if generated
        if (chunk.embedding) {
            console.log(`  Embedding dimensions: ${chunk.embedding.length}`);
            console.log(`  First 5 values: ${chunk.embedding.slice(0, 5)}`);
        }

        // Access chunk metadata
        console.log(`  Metadata:`, chunk.metadata);
    });
}
```

## Error Handling

All errors are instances of `Error` with descriptive messages:

```typescript
import { extractFile } from '@goldziher/kreuzberg';

try {
    const result = await extractFile('document.pdf');
} catch (error) {
    if (error instanceof Error) {
        // Check error type by message content
        if (error.message.includes('ValidationError')) {
            console.error('Validation failed:', error.message);
        } else if (error.message.includes('ParsingError')) {
            console.error('Parsing failed:', error.message);
        } else if (error.message.includes('OCRError')) {
            console.error('OCR failed:', error.message);
        } else if (error.message.includes('MissingDependencyError')) {
            console.error('Missing dependency:', error.message);
        } else {
            console.error('Extraction failed:', error.message);
        }
    } else {
        console.error('Unknown error:', error);
    }
}
```

## Advanced Topics

### Plugin Management

```typescript
import {
    registerPostProcessor,
    unregisterPostProcessor,
    clearPostProcessors,
    registerValidator,
    unregisterValidator,
    clearValidators,
} from '@goldziher/kreuzberg';

// Register custom PostProcessor
registerPostProcessor(new MyPostProcessor());

// Unregister by name
unregisterPostProcessor('my_processor');

// Clear all PostProcessors
clearPostProcessors();

// Register custom Validator
registerValidator(new MyValidator());

// Unregister by name
unregisterValidator('my_validator');

// Clear all Validators
clearValidators();
```

### Performance Tips

1. **Use batch processing** for multiple files with `batchExtractFiles`
2. **Enable caching** for repeated extractions with `useCache: true`
3. **Use async APIs** for I/O-bound workloads
4. **Configure OCR DPI** appropriately (300 DPI is usually sufficient)
5. **Use quality processing** only when needed (adds overhead)
6. **Process files in parallel** using `Promise.all` with batch APIs

### Language Detection

```typescript
import {
    ExtractionConfig,
    LanguageDetectionConfig,
    extractFile,
} from '@goldziher/kreuzberg';

const config = new ExtractionConfig({
    languageDetection: new LanguageDetectionConfig({
        minConfidence: 0.7, // Minimum confidence threshold
    }),
});

const result = await extractFile('document.pdf', null, config);
if (result.detectedLanguages) {
    console.log(`Detected languages: ${result.detectedLanguages}`);
}
```

### TypeScript Type Safety

Kreuzberg provides full TypeScript type definitions:

```typescript
import type {
    ExtractionResult,
    ExtractionConfig,
    PostProcessorProtocol,
    ValidatorProtocol,
    Metadata,
    TableData,
} from '@goldziher/kreuzberg';

// Type-safe plugin implementation
class MyProcessor implements PostProcessorProtocol {
    name(): string {
        return 'my_processor';
    }

    process(result: ExtractionResult): ExtractionResult {
        // Full type checking
        result.metadata.custom = 'value';
        return result;
    }
}
```

## Testing with Vitest/Jest

Example test using Vitest:

```typescript
import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import {
    extractFileSync,
    registerPostProcessor,
    clearPostProcessors,
    PostProcessorProtocol,
    ExtractionResult,
} from '@goldziher/kreuzberg';

describe('Custom PostProcessor', () => {
    class TestProcessor implements PostProcessorProtocol {
        name(): string {
            return 'test_processor';
        }

        process(result: ExtractionResult): ExtractionResult {
            result.metadata.test = 'success';
            return result;
        }
    }

    beforeEach(() => {
        clearPostProcessors();
        registerPostProcessor(new TestProcessor());
    });

    afterEach(() => {
        clearPostProcessors();
    });

    it('should apply custom processor', () => {
        const result = extractFileSync('test.txt');
        expect(result.metadata.test).toBe('success');
    });
});
```

## Next Steps

- **[Python Examples](python.md)** - Examples for Python
- **[Rust Examples](rust.md)** - Examples for Rust applications
- **[Plugin Development](../plugins/typescript-postprocessor.md)** - Deep dive into TypeScript plugins
- **[API Reference](../api/typescript.md)** - Complete TypeScript API documentation
