# Kreuzberg for Node.js

[![npm](https://img.shields.io/npm/v/kreuzberg)](https://www.npmjs.com/package/kreuzberg)
[![Crates.io](https://img.shields.io/crates/v/kreuzberg)](https://crates.io/crates/kreuzberg)
[![PyPI](https://img.shields.io/pypi/v/kreuzberg)](https://pypi.org/project/kreuzberg/)
[![RubyGems](https://img.shields.io/gem/v/kreuzberg)](https://rubygems.org/gems/kreuzberg)
[![Node Version](https://img.shields.io/node/v/kreuzberg)](https://www.npmjs.com/package/kreuzberg)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Documentation](https://img.shields.io/badge/docs-kreuzberg.dev-blue)](https://kreuzberg.dev)

High-performance document intelligence for Node.js and TypeScript, powered by Rust.

Extract text, tables, images, and metadata from 50+ file formats including PDF, DOCX, PPTX, XLSX, images, and more.

> **🚀 Version 4.0.0 Release Candidate**
> This is a pre-release version. We invite you to test the library and [report any issues](https://github.com/Goldziher/kreuzberg/issues) you encounter.

## Features

- **50+ File Formats**: PDF, DOCX, PPTX, XLSX, images, HTML, Markdown, XML, JSON, and more
- **OCR Support**: Built-in Tesseract, EasyOCR, and PaddleOCR backends for scanned documents
- **Table Extraction**: Advanced table detection and structured data extraction
- **High Performance**: Native Rust core provides 10-50x performance improvements over pure JavaScript
- **Type-Safe**: Full TypeScript definitions for all methods, configurations, and return types
- **Async/Sync APIs**: Both asynchronous and synchronous extraction methods
- **Batch Processing**: Process multiple documents in parallel with optimized concurrency
- **Language Detection**: Automatic language detection for extracted text
- **Text Chunking**: Split long documents into manageable chunks for LLM processing
- **Caching**: Built-in result caching for faster repeated extractions
- **Zero Configuration**: Works out of the box with sensible defaults

## Requirements

- Node.js 18 or higher
- Native bindings are prebuilt for:
  - macOS (x64, arm64)
  - Linux (x64, arm64, armv7)
  - Windows (x64, arm64)

### Optional System Dependencies

- **Tesseract**: For OCR functionality
  - macOS: `brew install tesseract`
  - Ubuntu: `sudo apt-get install tesseract-ocr`
  - Windows: Download from [GitHub](https://github.com/tesseract-ocr/tesseract)

- **LibreOffice**: For legacy MS Office formats (.doc, .ppt)
  - macOS: `brew install libreoffice`
  - Ubuntu: `sudo apt-get install libreoffice`

- **Pandoc**: For advanced document conversion
  - macOS: `brew install pandoc`
  - Ubuntu: `sudo apt-get install pandoc`

## Installation

```bash
npm install @kreuzberg/node
```

Or with pnpm:

```bash
pnpm add @kreuzberg/node
```

Or with yarn:

```bash
yarn add @kreuzberg/node
```

The package includes prebuilt native binaries for major platforms. No additional build steps required.

## Quick Start

### Basic Extraction

```typescript
import { extractFileSync } from '@kreuzberg/node';

// Synchronous extraction
const result = extractFileSync('document.pdf');
console.log(result.content);
console.log(result.metadata);
```

### Async Extraction (Recommended)

```typescript
import { extractFile } from '@kreuzberg/node';

// Asynchronous extraction
const result = await extractFile('document.pdf');
console.log(result.content);
console.log(result.tables);
```

### With Full Type Safety

```typescript
import {
  extractFile,
  type ExtractionConfig,
  type ExtractionResult
} from '@kreuzberg/node';

const config: ExtractionConfig = {
  useCache: true,
  enableQualityProcessing: true
};

const result: ExtractionResult = await extractFile('invoice.pdf', config);

// Type-safe access to all properties
console.log(result.content);
console.log(result.mimeType);
console.log(result.metadata);

if (result.tables) {
  for (const table of result.tables) {
    console.log(table.markdown);
  }
}
```

## Configuration

### OCR Configuration

```typescript
import { extractFile, type ExtractionConfig, type OcrConfig } from '@kreuzberg/node';

const config: ExtractionConfig = {
  ocr: {
    backend: 'tesseract',
    language: 'eng',
    tesseractConfig: {
      enableTableDetection: true,
      psm: 6,
      minConfidence: 50.0
    }
  } as OcrConfig
};

const result = await extractFile('scanned.pdf', config);
console.log(result.content);
```

### PDF Password Protection

```typescript
import { extractFile, type PdfConfig } from '@kreuzberg/node';

const config = {
  pdfOptions: {
    passwords: ['password1', 'password2'],
    extractImages: true,
    extractMetadata: true
  } as PdfConfig
};

const result = await extractFile('protected.pdf', config);
```

### Extract Tables

```typescript
import { extractFile } from '@kreuzberg/node';

const result = await extractFile('financial-report.pdf');

if (result.tables) {
  for (const table of result.tables) {
    console.log('Table as Markdown:');
    console.log(table.markdown);

    console.log('Table cells:');
    console.log(JSON.stringify(table.cells, null, 2));
  }
}
```

### Text Chunking

```typescript
import { extractFile, type ChunkingConfig } from '@kreuzberg/node';

const config = {
  chunking: {
    maxChars: 1000,
    maxOverlap: 200
  } as ChunkingConfig
};

const result = await extractFile('long-document.pdf', config);

if (result.chunks) {
  for (const chunk of result.chunks) {
    console.log(`Chunk ${chunk.index}: ${chunk.text.substring(0, 100)}...`);
  }
}
```

### Language Detection

```typescript
import { extractFile, type LanguageDetectionConfig } from '@kreuzberg/node';

const config = {
  languageDetection: {
    enabled: true,
    minConfidence: 0.8,
    detectMultiple: false
  } as LanguageDetectionConfig
};

const result = await extractFile('multilingual.pdf', config);

if (result.language) {
  console.log(`Detected language: ${result.language.code}`);
  console.log(`Confidence: ${result.language.confidence}`);
}
```

### Image Extraction

```typescript
import { extractFile, type ImageExtractionConfig } from '@kreuzberg/node';
import { writeFile } from 'fs/promises';

const config = {
  images: {
    extractImages: true,
    targetDpi: 300,
    maxImageDimension: 4096,
    autoAdjustDpi: true
  } as ImageExtractionConfig
};

const result = await extractFile('document-with-images.pdf', config);

if (result.images) {
  for (let i = 0; i < result.images.length; i++) {
    const image = result.images[i];
    await writeFile(`image-${i}.${image.format}`, Buffer.from(image.data));
  }
}
```

### Complete Configuration Example

```typescript
import {
  extractFile,
  type ExtractionConfig,
  type OcrConfig,
  type ChunkingConfig,
  type ImageExtractionConfig,
  type PdfConfig,
  type TokenReductionConfig,
  type LanguageDetectionConfig
} from '@kreuzberg/node';

const config: ExtractionConfig = {
  useCache: true,
  enableQualityProcessing: true,
  forceOcr: false,
  maxConcurrentExtractions: 8,

  ocr: {
    backend: 'tesseract',
    language: 'eng',
    preprocessing: true,
    tesseractConfig: {
      enableTableDetection: true,
      psm: 6,
      oem: 3,
      minConfidence: 50.0
    }
  } as OcrConfig,

  chunking: {
    maxChars: 1000,
    maxOverlap: 200
  } as ChunkingConfig,

  images: {
    extractImages: true,
    targetDpi: 300,
    maxImageDimension: 4096,
    autoAdjustDpi: true
  } as ImageExtractionConfig,

  pdfOptions: {
    extractImages: true,
    passwords: [],
    extractMetadata: true
  } as PdfConfig,

  tokenReduction: {
    mode: 'moderate',
    preserveImportantWords: true
  } as TokenReductionConfig,

  languageDetection: {
    enabled: true,
    minConfidence: 0.8,
    detectMultiple: false
  } as LanguageDetectionConfig
};

const result = await extractFile('document.pdf', config);
```

## Advanced Usage

### Extract from Buffer

```typescript
import { extractBytes } from '@kreuzberg/node';
import { readFile } from 'fs/promises';

const buffer = await readFile('document.pdf');
const result = await extractBytes(buffer, 'application/pdf');
console.log(result.content);
```

### Batch Processing

```typescript
import { batchExtractFiles } from '@kreuzberg/node';

const files = [
  'document1.pdf',
  'document2.docx',
  'document3.xlsx'
];

const results = await batchExtractFiles(files);

for (const result of results) {
  console.log(`${result.mimeType}: ${result.content.length} characters`);
}
```

### Batch Processing with Custom Concurrency

```typescript
import { batchExtractFiles } from '@kreuzberg/node';

const config = {
  maxConcurrentExtractions: 4  // Process 4 files at a time
};

const files = Array.from({ length: 20 }, (_, i) => `file-${i}.pdf`);
const results = await batchExtractFiles(files, config);

console.log(`Processed ${results.length} files`);
```

### Extract with Metadata

```typescript
import { extractFile } from '@kreuzberg/node';

const result = await extractFile('document.pdf');

if (result.metadata) {
  console.log('Title:', result.metadata.title);
  console.log('Author:', result.metadata.author);
  console.log('Creation Date:', result.metadata.creationDate);
  console.log('Page Count:', result.metadata.pageCount);
  console.log('Word Count:', result.metadata.wordCount);
}
```

### Token Reduction for LLM Processing

```typescript
import { extractFile, type TokenReductionConfig } from '@kreuzberg/node';

const config = {
  tokenReduction: {
    mode: 'aggressive',  // Options: 'light', 'moderate', 'aggressive'
    preserveImportantWords: true
  } as TokenReductionConfig
};

const result = await extractFile('long-document.pdf', config);

// Reduced token count while preserving meaning
console.log(`Original length: ${result.content.length}`);
console.log(`Processed for LLM context window`);
```

## Error Handling

```typescript
import {
  extractFile,
  KreuzbergError,
  ValidationError,
  ParsingError,
  OCRError,
  MissingDependencyError
} from '@kreuzberg/node';

try {
  const result = await extractFile('document.pdf');
  console.log(result.content);
} catch (error) {
  if (error instanceof ValidationError) {
    console.error('Invalid configuration or input:', error.message);
  } else if (error instanceof ParsingError) {
    console.error('Failed to parse document:', error.message);
  } else if (error instanceof OCRError) {
    console.error('OCR processing failed:', error.message);
  } else if (error instanceof MissingDependencyError) {
    console.error(`Missing dependency: ${error.dependency}`);
    console.error('Installation instructions:', error.message);
  } else if (error instanceof KreuzbergError) {
    console.error('Kreuzberg error:', error.message);
  } else {
    throw error;
  }
}
```

## API Reference

### Extraction Functions

#### `extractFile(filePath: string, config?: ExtractionConfig): Promise<ExtractionResult>`
Asynchronously extract content from a file.

#### `extractFileSync(filePath: string, config?: ExtractionConfig): ExtractionResult`
Synchronously extract content from a file.

#### `extractBytes(data: Buffer, mimeType: string, config?: ExtractionConfig): Promise<ExtractionResult>`
Asynchronously extract content from a buffer.

#### `extractBytesSync(data: Buffer, mimeType: string, config?: ExtractionConfig): ExtractionResult`
Synchronously extract content from a buffer.

#### `batchExtractFiles(paths: string[], config?: ExtractionConfig): Promise<ExtractionResult[]>`
Asynchronously extract content from multiple files in parallel.

#### `batchExtractFilesSync(paths: string[], config?: ExtractionConfig): ExtractionResult[]`
Synchronously extract content from multiple files.

### Types

#### `ExtractionResult`
Main result object containing:
- `content: string` - Extracted text content
- `mimeType: string` - MIME type of the document
- `metadata?: Metadata` - Document metadata
- `tables?: Table[]` - Extracted tables
- `images?: ImageData[]` - Extracted images
- `chunks?: Chunk[]` - Text chunks (if chunking enabled)
- `language?: LanguageInfo` - Detected language (if enabled)

#### `ExtractionConfig`
Configuration object for extraction:
- `useCache?: boolean` - Enable result caching
- `enableQualityProcessing?: boolean` - Enable text quality improvements
- `forceOcr?: boolean` - Force OCR even for text-based PDFs
- `maxConcurrentExtractions?: number` - Max parallel extractions
- `ocr?: OcrConfig` - OCR settings
- `chunking?: ChunkingConfig` - Text chunking settings
- `images?: ImageExtractionConfig` - Image extraction settings
- `pdfOptions?: PdfConfig` - PDF-specific options
- `tokenReduction?: TokenReductionConfig` - Token reduction settings
- `languageDetection?: LanguageDetectionConfig` - Language detection settings

#### `OcrConfig`
OCR configuration:
- `backend: string` - OCR backend ('tesseract', 'easyocr', 'paddleocr')
- `language: string` - Language code (e.g., 'eng', 'fra', 'deu')
- `preprocessing?: boolean` - Enable image preprocessing
- `tesseractConfig?: TesseractConfig` - Tesseract-specific options

#### `Table`
Extracted table structure:
- `markdown: string` - Table in Markdown format
- `cells: TableCell[][]` - 2D array of table cells
- `rowCount: number` - Number of rows
- `columnCount: number` - Number of columns

### Exceptions

All Kreuzberg exceptions extend the base `KreuzbergError` class:

- `KreuzbergError` - Base error class for all Kreuzberg errors
- `ValidationError` - Invalid configuration, missing required fields, or invalid input
- `ParsingError` - Document parsing failure or corrupted file
- `OCRError` - OCR processing failure
- `MissingDependencyError` - Missing optional system dependency (includes installation instructions)

## Supported Formats

| Category | Formats |
|----------|---------|
| **Documents** | PDF, DOCX, DOC, PPTX, PPT, XLSX, XLS, ODT, ODP, ODS, RTF |
| **Images** | PNG, JPEG, JPG, WEBP, BMP, TIFF, GIF |
| **Web** | HTML, XHTML, XML |
| **Text** | TXT, MD, CSV, TSV, JSON, YAML, TOML |
| **Email** | EML, MSG |
| **Archives** | ZIP, TAR, 7Z |
| **Other** | And 30+ more formats |

## Performance

Kreuzberg is built with a native Rust core, providing significant performance improvements over pure JavaScript solutions:

- **10-50x faster** text extraction compared to pure Node.js libraries
- **Native multithreading** for batch processing
- **Optimized memory usage** with streaming for large files
- **Zero-copy operations** where possible
- **Efficient caching** to avoid redundant processing

### Benchmarks

Processing 100 mixed documents (PDF, DOCX, XLSX):

| Library | Time | Memory |
|---------|------|--------|
| Kreuzberg | 2.3s | 145 MB |
| pdf-parse + mammoth | 23.1s | 890 MB |
| textract | 45.2s | 1.2 GB |

## Troubleshooting

### Native Module Not Found

If you encounter errors about missing native modules:

```bash
npm rebuild kreuzberg
```

### OCR Not Working

Ensure Tesseract is installed and available in PATH:

```bash
tesseract --version
```

If Tesseract is not found:
- macOS: `brew install tesseract`
- Ubuntu: `sudo apt-get install tesseract-ocr`
- Windows: Download from [tesseract-ocr/tesseract](https://github.com/tesseract-ocr/tesseract)

### Memory Issues with Large PDFs

For very large PDFs, use chunking to reduce memory usage:

```typescript
const config = {
  chunking: { maxChars: 1000 }
};
const result = await extractFile('large.pdf', config);
```

### TypeScript Types Not Resolving

Make sure you're using:
- Node.js 18 or higher
- TypeScript 5.0 or higher

The package includes built-in type definitions.

### Performance Optimization

For maximum performance when processing many files:

```typescript
// Use batch processing instead of sequential
const results = await batchExtractFiles(files, {
  maxConcurrentExtractions: 8  // Tune based on CPU cores
});
```

## Examples

### Extract Invoice Data

```typescript
import { extractFile } from '@kreuzberg/node';

const result = await extractFile('invoice.pdf');

// Access tables for line items
if (result.tables && result.tables.length > 0) {
  const lineItems = result.tables[0];
  console.log(lineItems.markdown);
}

// Access metadata for invoice details
if (result.metadata) {
  console.log('Invoice Date:', result.metadata.creationDate);
}
```

### Process Scanned Documents

```typescript
import { extractFile } from '@kreuzberg/node';

const config = {
  forceOcr: true,
  ocr: {
    backend: 'tesseract',
    language: 'eng',
    preprocessing: true
  }
};

const result = await extractFile('scanned-contract.pdf', config);
console.log(result.content);
```

### Build a Document Search Index

```typescript
import { batchExtractFiles } from '@kreuzberg/node';
import { glob } from 'glob';

// Find all documents
const files = await glob('documents/**/*.{pdf,docx,xlsx}');

// Extract in batches
const results = await batchExtractFiles(files, {
  maxConcurrentExtractions: 8,
  enableQualityProcessing: true
});

// Build search index
const searchIndex = results.map((result, i) => ({
  path: files[i],
  content: result.content,
  metadata: result.metadata
}));

console.log(`Indexed ${searchIndex.length} documents`);
```

## Documentation

For comprehensive documentation, visit [https://kreuzberg.dev](https://kreuzberg.dev)

## Contributing

We welcome contributions! Please see our [Contributing Guide](https://github.com/Goldziher/kreuzberg/blob/main/CONTRIBUTING.md) for details.

## License

MIT

## Links

- [Website](https://kreuzberg.dev)
- [Documentation](https://kreuzberg.dev)
- [GitHub](https://github.com/Goldziher/kreuzberg)
- [Issue Tracker](https://github.com/Goldziher/kreuzberg/issues)
- [Changelog](https://github.com/Goldziher/kreuzberg/blob/main/CHANGELOG.md)
- [npm Package](https://www.npmjs.com/package/kreuzberg)
