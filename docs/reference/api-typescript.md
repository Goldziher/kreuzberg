# TypeScript API Reference

Complete reference for the Kreuzberg TypeScript/Node.js API.

## Installation

```bash
npm install kreuzberg
```

**Or with other package managers:**

```bash
# Yarn
yarn add kreuzberg

# pnpm
pnpm add kreuzberg
```

## Core Functions

### extractFileSync()

Extract content from a file (synchronous).

**Signature:**

```typescript
function extractFileSync(
  filePath: string,
  mimeType: string | null = null,
  config: ExtractionConfig | null = null
): ExtractionResult
```

**Parameters:**

- `filePath` (string): Path to the file to extract
- `mimeType` (string | null): Optional MIME type hint. If null, MIME type is auto-detected from file extension and content
- `config` (ExtractionConfig | null): Extraction configuration. Uses defaults if null

**Returns:**

- `ExtractionResult`: Extraction result containing content, metadata, and tables

**Throws:**

- `Error`: Base error for all extraction failures (validation, parsing, OCR, etc.)

**Example - Basic usage:**

```typescript
import { extractFileSync } from 'kreuzberg';

const result = extractFileSync('document.pdf');
console.log(result.content);
console.log(`Pages: ${result.metadata.pageCount}`);
```

**Example - With OCR:**

```typescript
import { extractFileSync } from 'kreuzberg';

const config = {
  ocr: {
    backend: 'tesseract',
    language: 'eng'
  }
};
const result = extractFileSync('scanned.pdf', null, config);
```

**Example - With explicit MIME type:**

```typescript
import { extractFileSync } from 'kreuzberg';

const result = extractFileSync('document.pdf', 'application/pdf');
```

---

### extractFile()

Extract content from a file (asynchronous).

**Signature:**

```typescript
async function extractFile(
  filePath: string,
  mimeType: string | null = null,
  config: ExtractionConfig | null = null
): Promise<ExtractionResult>
```

**Parameters:**

Same as [`extractFileSync()`](#extractfilesync).

**Returns:**

- `Promise<ExtractionResult>`: Promise resolving to extraction result

**Examples:**

```typescript
import { extractFile } from 'kreuzberg';

async function main() {
  const result = await extractFile('document.pdf');
  console.log(result.content);
}

main();
```

---

### extractBytesSync()

Extract content from bytes (synchronous).

**Signature:**

```typescript
function extractBytesSync(
  data: Uint8Array,
  mimeType: string,
  config: ExtractionConfig | null = null
): ExtractionResult
```

**Parameters:**

- `data` (Uint8Array): File content as Uint8Array
- `mimeType` (string): MIME type of the data (required for format detection)
- `config` (ExtractionConfig | null): Extraction configuration. Uses defaults if null

**Returns:**

- `ExtractionResult`: Extraction result containing content, metadata, and tables

**Examples:**

```typescript
import { extractBytesSync } from 'kreuzberg';
import { readFileSync } from 'fs';

const data = readFileSync('document.pdf');
const result = extractBytesSync(data, 'application/pdf');
console.log(result.content);
```

---

### extractBytes()

Extract content from bytes (asynchronous).

**Signature:**

```typescript
async function extractBytes(
  data: Uint8Array,
  mimeType: string,
  config: ExtractionConfig | null = null
): Promise<ExtractionResult>
```

**Parameters:**

Same as [`extractBytesSync()`](#extractbytessync).

**Returns:**

- `Promise<ExtractionResult>`: Promise resolving to extraction result

---

### batchExtractFilesSync()

Extract content from multiple files in parallel (synchronous).

**Signature:**

```typescript
function batchExtractFilesSync(
  paths: string[],
  config: ExtractionConfig | null = null
): ExtractionResult[]
```

**Parameters:**

- `paths` (string[]): Array of file paths to extract
- `config` (ExtractionConfig | null): Extraction configuration applied to all files

**Returns:**

- `ExtractionResult[]`: Array of extraction results (one per file)

**Examples:**

```typescript
import { batchExtractFilesSync } from 'kreuzberg';

const paths = ['doc1.pdf', 'doc2.docx', 'doc3.xlsx'];
const results = batchExtractFilesSync(paths);

results.forEach((result, i) => {
  console.log(`${paths[i]}: ${result.content.length} characters`);
});
```

---

### batchExtractFiles()

Extract content from multiple files in parallel (asynchronous).

**Signature:**

```typescript
async function batchExtractFiles(
  paths: string[],
  config: ExtractionConfig | null = null
): Promise<ExtractionResult[]>
```

**Parameters:**

Same as [`batchExtractFilesSync()`](#batchextractfilessync).

**Returns:**

- `Promise<ExtractionResult[]>`: Promise resolving to array of extraction results

**Examples:**

```typescript
import { batchExtractFiles } from 'kreuzberg';

const files = ['doc1.pdf', 'doc2.docx', 'doc3.xlsx'];
const results = await batchExtractFiles(files);

for (const result of results) {
  console.log(result.content);
}
```

---

### batchExtractBytesSync()

Extract content from multiple byte arrays in parallel (synchronous).

**Signature:**

```typescript
function batchExtractBytesSync(
  dataList: Uint8Array[],
  mimeTypes: string[],
  config: ExtractionConfig | null = null
): ExtractionResult[]
```

**Parameters:**

- `dataList` (Uint8Array[]): Array of file contents as Uint8Array
- `mimeTypes` (string[]): Array of MIME types (one per data item, same length as dataList)
- `config` (ExtractionConfig | null): Extraction configuration applied to all items

**Returns:**

- `ExtractionResult[]`: Array of extraction results (one per data item)

---

### batchExtractBytes()

Extract content from multiple byte arrays in parallel (asynchronous).

**Signature:**

```typescript
async function batchExtractBytes(
  dataList: Uint8Array[],
  mimeTypes: string[],
  config: ExtractionConfig | null = null
): Promise<ExtractionResult[]>
```

**Parameters:**

Same as [`batchExtractBytesSync()`](#batchextractbytessync).

**Returns:**

- `Promise<ExtractionResult[]>`: Promise resolving to array of extraction results

---

## Configuration

### ExtractionConfig

Main configuration interface for extraction operations.

**Type Definition:**

```typescript
interface ExtractionConfig {
  ocr?: OcrConfig | null;
  forceOcr?: boolean;
  pdfOptions?: PdfConfig | null;
  chunking?: ChunkingConfig | null;
  languageDetection?: LanguageDetectionConfig | null;
  tokenReduction?: TokenReductionConfig | null;
  imageExtraction?: ImageExtractionConfig | null;
  postProcessor?: PostProcessorConfig | null;
}
```

**Fields:**

- `ocr` (OcrConfig | null): OCR configuration. Default: null (no OCR)
- `forceOcr` (boolean): Force OCR even for text-based PDFs. Default: false
- `pdfOptions` (PdfConfig | null): PDF-specific configuration. Default: null
- `chunking` (ChunkingConfig | null): Text chunking configuration. Default: null
- `languageDetection` (LanguageDetectionConfig | null): Language detection configuration. Default: null
- `tokenReduction` (TokenReductionConfig | null): Token reduction configuration. Default: null
- `imageExtraction` (ImageExtractionConfig | null): Image extraction from documents. Default: null
- `postProcessor` (PostProcessorConfig | null): Post-processing configuration. Default: null

**Example:**

```typescript
import { extractFileSync, ExtractionConfig } from 'kreuzberg';

const config: ExtractionConfig = {
  ocr: {
    backend: 'tesseract',
    language: 'eng'
  },
  forceOcr: false,
  pdfOptions: {
    passwords: ['password1', 'password2'],
    extractImages: true
  }
};

const result = extractFileSync('document.pdf', null, config);
```

---

### OcrConfig

OCR processing configuration.

**Type Definition:**

```typescript
interface OcrConfig {
  backend: string;
  language: string;
  tesseractConfig?: TesseractConfig | null;
}
```

**Fields:**

- `backend` (string): OCR backend to use. Options: "tesseract", "guten-ocr". Default: "tesseract"
- `language` (string): Language code for OCR (ISO 639-3). Default: "eng"
- `tesseractConfig` (TesseractConfig | null): Tesseract-specific configuration. Default: null

**Example:**

```typescript
const ocrConfig: OcrConfig = {
  backend: 'tesseract',
  language: 'eng'
};
```

---

### TesseractConfig

Tesseract OCR backend configuration.

**Type Definition:**

```typescript
interface TesseractConfig {
  psm?: number;
  oem?: number;
  enableTableDetection?: boolean;
  tesseditCharWhitelist?: string | null;
  tesseditCharBlacklist?: string | null;
}
```

**Fields:**

- `psm` (number): Page segmentation mode (0-13). Default: 3 (auto)
- `oem` (number): OCR engine mode (0-3). Default: 3 (LSTM only)
- `enableTableDetection` (boolean): Enable table detection and extraction. Default: false
- `tesseditCharWhitelist` (string | null): Character whitelist (e.g., "0123456789" for digits only). Default: null
- `tesseditCharBlacklist` (string | null): Character blacklist. Default: null

**Example:**

```typescript
const config: ExtractionConfig = {
  ocr: {
    backend: 'tesseract',
    language: 'eng',
    tesseractConfig: {
      psm: 6,
      enableTableDetection: true,
      tesseditCharWhitelist: '0123456789'
    }
  }
};
```

---

### PdfConfig

PDF-specific configuration.

**Type Definition:**

```typescript
interface PdfConfig {
  passwords?: string[] | null;
  extractImages?: boolean;
  imageDpi?: number;
}
```

**Fields:**

- `passwords` (string[] | null): List of passwords to try for encrypted PDFs. Default: null
- `extractImages` (boolean): Extract images from PDF. Default: false
- `imageDpi` (number): DPI for image extraction. Default: 300

**Example:**

```typescript
const pdfConfig: PdfConfig = {
  passwords: ['password1', 'password2'],
  extractImages: true,
  imageDpi: 300
};
```

---

### ChunkingConfig

Text chunking configuration for splitting long documents.

**Type Definition:**

```typescript
interface ChunkingConfig {
  chunkSize?: number;
  chunkOverlap?: number;
  chunkingStrategy?: string;
}
```

**Fields:**

- `chunkSize` (number): Maximum chunk size in tokens. Default: 512
- `chunkOverlap` (number): Overlap between chunks in tokens. Default: 50
- `chunkingStrategy` (string): Chunking strategy. Options: "fixed", "semantic". Default: "fixed"

---

### LanguageDetectionConfig

Language detection configuration.

**Type Definition:**

```typescript
interface LanguageDetectionConfig {
  enabled?: boolean;
  confidenceThreshold?: number;
}
```

**Fields:**

- `enabled` (boolean): Enable language detection. Default: true
- `confidenceThreshold` (number): Minimum confidence threshold (0.0-1.0). Default: 0.5

---

### ImageExtractionConfig

Image extraction configuration.

**Type Definition:**

```typescript
interface ImageExtractionConfig {
  enabled?: boolean;
  minWidth?: number;
  minHeight?: number;
}
```

**Fields:**

- `enabled` (boolean): Enable image extraction from documents. Default: false
- `minWidth` (number): Minimum image width in pixels. Default: 100
- `minHeight` (number): Minimum image height in pixels. Default: 100

---

### TokenReductionConfig

Token reduction configuration for compressing extracted text.

**Type Definition:**

```typescript
interface TokenReductionConfig {
  enabled?: boolean;
  strategy?: string;
}
```

**Fields:**

- `enabled` (boolean): Enable token reduction. Default: false
- `strategy` (string): Reduction strategy. Options: "whitespace", "stemming". Default: "whitespace"

---

### PostProcessorConfig

Post-processing configuration.

**Type Definition:**

```typescript
interface PostProcessorConfig {
  enabled?: boolean;
  processors?: string[] | null;
}
```

**Fields:**

- `enabled` (boolean): Enable post-processing. Default: true
- `processors` (string[] | null): List of processor names to enable. Default: null (all registered processors)

---

## Results & Types

### ExtractionResult

Result object returned by all extraction functions.

**Type Definition:**

```typescript
interface ExtractionResult {
  content: string;
  mimeType: string;
  metadata: Metadata;
  tables: Table[];
  detectedLanguages: string[] | null;
}
```

**Fields:**

- `content` (string): Extracted text content
- `mimeType` (string): MIME type of the processed document
- `metadata` (Metadata): Document metadata (format-specific fields)
- `tables` (Table[]): Array of extracted tables
- `detectedLanguages` (string[] | null): Array of detected language codes (ISO 639-1) if language detection is enabled

**Example:**

```typescript
const result = extractFileSync('document.pdf');

console.log(`Content: ${result.content}`);
console.log(`MIME type: ${result.mimeType}`);
console.log(`Page count: ${result.metadata.pageCount}`);
console.log(`Tables: ${result.tables.length}`);

if (result.detectedLanguages) {
  console.log(`Languages: ${result.detectedLanguages.join(', ')}`);
}
```

---

### Metadata

Document metadata with format-specific fields.

**Type Definition:**

```typescript
interface Metadata {
  // Common fields
  language?: string;
  date?: string;
  subject?: string;
  formatType?: string;

  // PDF-specific fields
  title?: string;
  author?: string;
  pageCount?: number;
  creationDate?: string;
  modificationDate?: string;
  creator?: string;
  producer?: string;
  keywords?: string;

  // Excel-specific fields
  sheetCount?: number;
  sheetNames?: string[];

  // Email-specific fields
  fromEmail?: string;
  fromName?: string;
  toEmails?: string[];
  ccEmails?: string[];
  bccEmails?: string[];
  messageId?: string;
  attachments?: string[];

  // Additional fields...
  [key: string]: any;
}
```

**Common Fields:**

- `language` (string): Document language (ISO 639-1 code)
- `date` (string): Document date (ISO 8601 format)
- `subject` (string): Document subject
- `formatType` (string): Format discriminator ("pdf", "excel", "email", etc.)

**PDF-Specific Fields** (when `formatType === "pdf"`):

- `title` (string): PDF title
- `author` (string): PDF author
- `pageCount` (number): Number of pages
- `creationDate` (string): Creation date (ISO 8601)
- `modificationDate` (string): Modification date (ISO 8601)
- `creator` (string): Creator application
- `producer` (string): Producer application
- `keywords` (string): PDF keywords

**Example:**

```typescript
const result = extractFileSync('document.pdf');
const metadata = result.metadata;

if (metadata.formatType === 'pdf') {
  console.log(`Title: ${metadata.title}`);
  console.log(`Author: ${metadata.author}`);
  console.log(`Pages: ${metadata.pageCount}`);
}
```

See the Types Reference for complete metadata field documentation.

---

### Table

Extracted table structure.

**Type Definition:**

```typescript
interface Table {
  cells: string[][];
  markdown: string;
  pageNumber: number;
}
```

**Fields:**

- `cells` (`string[][]`): 2D array of table cells (rows x columns)
- `markdown` (string): Table rendered as markdown
- `pageNumber` (number): Page number where table was found

**Example:**

```typescript
const result = extractFileSync('invoice.pdf');

for (const table of result.tables) {
  console.log(`Table on page ${table.pageNumber}:`);
  console.log(table.markdown);
  console.log();
}
```

---

## Extensibility

### Custom Post-Processors

Create custom post-processors to add processing logic to the extraction pipeline.

**Protocol:**

```typescript
interface PostProcessorProtocol {
  name(): string;
  process(result: ExtractionResult): ExtractionResult;
  processingStage(): string;
}
```

**Example:**

```typescript
import { registerPostProcessor, extractFileSync } from 'kreuzberg';

class CustomProcessor implements PostProcessorProtocol {
  name(): string {
    return 'custom_processor';
  }

  process(result: ExtractionResult): ExtractionResult {
    // Add custom field to metadata
    result.metadata.customField = 'custom_value';
    return result;
  }

  processingStage(): string {
    return 'middle';
  }
}

// Register the processor
registerPostProcessor(new CustomProcessor());

// Now all extractions will use this processor
const result = extractFileSync('document.pdf');
console.log(result.metadata.customField); // "custom_value"
```

**Managing Processors:**

```typescript
import {
  registerPostProcessor,
  unregisterPostProcessor,
  clearPostProcessors
} from 'kreuzberg';

// Register
registerPostProcessor(new CustomProcessor());

// Unregister by name
unregisterPostProcessor('custom_processor');

// Clear all processors
clearPostProcessors();
```

---

### Custom Validators

Create custom validators to validate extraction results.

**Protocol:**

```typescript
interface ValidatorProtocol {
  name(): string;
  validate(result: ExtractionResult): void;
}
```

**Functions:**

```typescript
import {
  registerValidator,
  unregisterValidator,
  clearValidators
} from 'kreuzberg';

// Register a validator
registerValidator(validator);

// Unregister by name
unregisterValidator('validator_name');

// Clear all validators
clearValidators();
```

---

### Custom OCR Backends

Register custom OCR backends for image and PDF processing.

**Example with Guten-OCR:**

```typescript
import { GutenOcrBackend, registerOcrBackend } from 'kreuzberg';

// Register Guten-OCR backend
const gutenOcr = new GutenOcrBackend();
registerOcrBackend(gutenOcr);

// Now you can use it in config
const config = {
  ocr: {
    backend: 'guten-ocr',
    language: 'eng'
  }
};
```

---

## Error Handling

All errors are thrown as standard JavaScript `Error` objects with descriptive messages.

**Example:**

```typescript
import { extractFileSync } from 'kreuzberg';

try {
  const result = extractFileSync('document.pdf');
  console.log(result.content);
} catch (error) {
  console.error(`Extraction failed: ${error.message}`);

  // Check error details
  if (error.message.includes('file not found')) {
    console.error('File does not exist');
  } else if (error.message.includes('parsing')) {
    console.error('Failed to parse document');
  } else if (error.message.includes('OCR')) {
    console.error('OCR processing failed');
  }
}
```

See [Error Handling Reference](errors.md) for detailed error documentation.

---

## Type Exports

All types are exported for use in your TypeScript code:

```typescript
import type {
  ExtractionConfig,
  ExtractionResult,
  OcrConfig,
  TesseractConfig,
  PdfConfig,
  ChunkingConfig,
  LanguageDetectionConfig,
  ImageExtractionConfig,
  TokenReductionConfig,
  PostProcessorConfig,
  Table,
  Metadata,
  PostProcessorProtocol,
  ValidatorProtocol,
  OcrBackendProtocol
} from 'kreuzberg';
```

---

## Performance Recommendations

### Batch Processing

For processing multiple documents, **always use batch APIs**:

```typescript
//  Good - Uses batch API
const results = await batchExtractFiles(['doc1.pdf', 'doc2.pdf', 'doc3.pdf']);

// L Bad - Multiple individual calls
const results = [];
for (const file of files) {
  results.push(await extractFile(file));
}
```

**Benefits of batch APIs:**

- Parallel processing in Rust
- Better memory management
- Optimal resource utilization

### Sync vs Async

- Use **async functions** (`extractFile`, `batchExtractFiles`) for I/O-bound operations
- Use **sync functions** (`extractFileSync`, `batchExtractFilesSync`) for simple scripts or CLI tools

---

## System Requirements

**Node.js:** 16.x or higher

**Native Dependencies:**

- Tesseract OCR (for OCR support): `brew install tesseract` (macOS) or `apt-get install tesseract-ocr` (Ubuntu)
- LibreOffice (for legacy Office formats): `brew install libreoffice` (macOS) or `apt-get install libreoffice` (Ubuntu)

**Platforms:**

- Linux (x64, arm64)
- macOS (x64, arm64)
- Windows (x64)
