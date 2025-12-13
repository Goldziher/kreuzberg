# WebAssembly API Reference

Complete reference for the Kreuzberg WebAssembly binding (`@kreuzberg/wasm`).

The WASM binding provides a browser-compatible, runtime-agnostic interface to Kreuzberg's document extraction capabilities. It works in browsers, Node.js, Deno, Bun, and Cloudflare Workers.

## Installation

```bash title="npm"
npm install @kreuzberg/wasm
```

**Or with other package managers:**

```bash title="Terminal"
# Yarn
yarn add @kreuzberg/wasm

# pnpm
pnpm add @kreuzberg/wasm
```

### Deno

```typescript title="TypeScript"
import { extractBytes, initWasm } from "npm:@kreuzberg/wasm@^4.0.0";
```

## Module Initialization

### initWasm()

Initialize the WASM module. This must be called once before using any extraction functions.

**Signature:**

```typescript title="TypeScript"
async function initWasm(): Promise<void>
```

**Throws:**

- `Error`: If WASM module fails to load or is not supported in the current environment

**Example - Basic initialization:**

```typescript title="init_wasm.ts"
import { initWasm } from '@kreuzberg/wasm';

async function main() {
  await initWasm();
  // Now you can use extraction functions
}

main().catch(console.error);
```

**Example - With error handling:**

```typescript title="init_with_error_handling.ts"
import { initWasm, getWasmCapabilities } from '@kreuzberg/wasm';

async function initializeKreuzberg() {
  const caps = getWasmCapabilities();
  if (!caps.hasWasm) {
    throw new Error('WebAssembly is not supported in this environment');
  }

  try {
    await initWasm();
    console.log('Kreuzberg initialized successfully');
  } catch (error) {
    console.error('Failed to initialize Kreuzberg:', error);
    throw error;
  }
}

initializeKreuzberg().catch(console.error);
```

---

### isInitialized()

Check if the WASM module is initialized.

**Signature:**

```typescript title="TypeScript"
function isInitialized(): boolean
```

**Returns:**

- `boolean`: True if WASM module is initialized, false otherwise

**Example:**

```typescript title="check_init.ts"
import { isInitialized, initWasm } from '@kreuzberg/wasm';

if (!isInitialized()) {
  await initWasm();
}
```

---

### getVersion()

Get the WASM module version.

**Signature:**

```typescript title="TypeScript"
function getVersion(): string
```

**Returns:**

- `string`: The version string of the WASM module

**Throws:**

- `Error`: If WASM module is not initialized

**Example:**

```typescript title="get_version.ts"
import { initWasm, getVersion } from '@kreuzberg/wasm';

await initWasm();
const version = getVersion();
console.log(`Using Kreuzberg ${version}`);
```

---

### getInitializationError()

Get the initialization error if module failed to load. Used for debugging initialization issues.

**Signature:**

```typescript title="TypeScript"
function getInitializationError(): Error | null
```

**Returns:**

- `Error | null`: The error that occurred during initialization, or null if no error

---

## Core Extraction Functions

### extractBytes()

Extract content from document bytes asynchronously.

**Signature:**

```typescript title="TypeScript"
async function extractBytes(
  data: Uint8Array,
  mimeType: string,
  config?: ExtractionConfig | null
): Promise<ExtractionResult>
```

**Parameters:**

- `data` (Uint8Array): The document bytes to extract from
- `mimeType` (string): MIME type of the document (e.g., 'application/pdf', 'image/jpeg'). Required.
- `config` (ExtractionConfig | null): Optional extraction configuration. Uses defaults if not provided.

**Returns:**

- `Promise<ExtractionResult>`: Extraction result containing content, metadata, tables, images, chunks, and more

**Throws:**

- `Error`: If WASM module is not initialized, document data is empty, MIME type is missing, or extraction fails

**Example - Extract PDF:**

```typescript title="extract_pdf.ts"
import { initWasm, extractBytes } from '@kreuzberg/wasm';

await initWasm();

const pdfBytes = new Uint8Array(buffer);
const result = await extractBytes(pdfBytes, 'application/pdf');
console.log(result.content);
console.log(`Found ${result.tables?.length ?? 0} tables`);
```

**Example - Extract with configuration:**

```typescript title="extract_with_config.ts"
import { initWasm, extractBytes } from '@kreuzberg/wasm';
import type { ExtractionConfig } from '@kreuzberg/wasm';

await initWasm();

const config: ExtractionConfig = {
  ocr: {
    backend: 'tesseract-wasm',
    language: 'deu' // German
  },
  images: {
    extractImages: true,
    targetDpi: 200
  }
};

const result = await extractBytes(pdfBytes, 'application/pdf', config);
```

**Example - Extract from File in browser:**

```typescript title="extract_from_file_browser.ts"
import { initWasm, extractBytes } from '@kreuzberg/wasm';
import { fileToUint8Array } from '@kreuzberg/wasm/adapters/wasm-adapter';

await initWasm();

const file = inputEvent.target.files[0];
const bytes = await fileToUint8Array(file);
const result = await extractBytes(bytes, file.type);
console.log(result.content);
```

---

### extractFile()

Extract content from a file on the file system (Node.js, Deno, Bun only).

**Signature:**

```typescript title="TypeScript"
async function extractFile(
  path: string,
  mimeType?: string | null,
  config?: ExtractionConfig | null
): Promise<ExtractionResult>
```

**Parameters:**

- `path` (string): Path to the file to extract from. Required.
- `mimeType` (string | null): Optional MIME type. If not provided, will be auto-detected from file content and extension.
- `config` (ExtractionConfig | null): Optional extraction configuration

**Returns:**

- `Promise<ExtractionResult>`: Extraction result

**Throws:**

- `Error`: If WASM module is not initialized, file path is missing, file doesn't exist, runtime is not supported (browser), or extraction fails

**Example - Extract with auto-detection:**

```typescript title="extract_file_auto.ts"
import { extractFile } from '@kreuzberg/wasm';

const result = await extractFile('./document.pdf');
console.log(result.content);
```

**Example - Extract with explicit MIME type:**

```typescript title="extract_file_explicit.ts"
import { extractFile } from '@kreuzberg/wasm';

const result = await extractFile('./document.docx', 'application/vnd.openxmlformats-officedocument.wordprocessingml.document');
console.log(result.content);
```

**Example - Extract with configuration:**

```typescript title="extract_file_config.ts"
import { extractFile } from '@kreuzberg/wasm';

const result = await extractFile('./report.xlsx', null, {
  chunking: {
    maxChars: 1000
  }
});
```

---

### extractFromFile()

Extract content from a File or Blob (browser-friendly wrapper).

Convenience function that combines `fileToUint8Array()` and `extractBytes()` for streamlined browser usage.

**Signature:**

```typescript title="TypeScript"
async function extractFromFile(
  file: File | Blob,
  mimeType?: string | null,
  config?: ExtractionConfig | null
): Promise<ExtractionResult>
```

**Parameters:**

- `file` (File | Blob): The File or Blob to extract from. Required.
- `mimeType` (string | null): Optional MIME type. If not provided, uses `file.type` for File objects, defaults to 'application/octet-stream' for Blob.
- `config` (ExtractionConfig | null): Optional extraction configuration

**Returns:**

- `Promise<ExtractionResult>`: Extraction result

**Throws:**

- `Error`: If WASM module is not initialized or extraction fails

**Example - Simple file input:**

```typescript title="extract_from_file.ts"
import { initWasm, extractFromFile } from '@kreuzberg/wasm';

await initWasm();

const fileInput = document.getElementById('file') as HTMLInputElement;
fileInput.addEventListener('change', async (e) => {
  const file = e.target.files?.[0];
  if (file) {
    const result = await extractFromFile(file);
    console.log(result.content);
  }
});
```

**Example - With configuration:**

```typescript title="extract_from_file_config.ts"
import { extractFromFile } from '@kreuzberg/wasm';

const result = await extractFromFile(file, file.type, {
  chunking: { maxChars: 1000 },
  images: { extractImages: true }
});
```

---

### batchExtractBytes()

Extract content from multiple byte arrays in parallel.

**Signature:**

```typescript title="TypeScript"
async function batchExtractBytes(
  dataList: Uint8Array[],
  mimeTypes: string[],
  config?: ExtractionConfig | null
): Promise<ExtractionResult[]>
```

**Parameters:**

- `dataList` (Uint8Array[]): Array of document bytes to extract from. Required.
- `mimeTypes` (string[]): Array of MIME types corresponding to each document. Must match length of `dataList`. Required.
- `config` (ExtractionConfig | null): Optional extraction configuration applied to all documents

**Returns:**

- `Promise<ExtractionResult[]>`: Array of extraction results in the same order as input

**Throws:**

- `Error`: If WASM module is not initialized or any extraction fails

**Example:**

```typescript title="batch_extract_bytes.ts"
import { initWasm, batchExtractBytes } from '@kreuzberg/wasm';

await initWasm();

const dataList = [pdfBytes1, pdfBytes2, pdfBytes3];
const mimeTypes = ['application/pdf', 'application/pdf', 'application/pdf'];

const results = await batchExtractBytes(dataList, mimeTypes, {
  extract_tables: true
});

for (const result of results) {
  console.log(`${result.mimeType}: ${result.content.length} characters`);
}
```

---

### batchExtractFiles()

Extract content from multiple browser File objects in parallel.

**Signature:**

```typescript title="TypeScript"
async function batchExtractFiles(
  files: File[],
  config?: ExtractionConfig | null
): Promise<ExtractionResult[]>
```

**Parameters:**

- `files` (File[]): Array of File objects to extract from. Required.
- `config` (ExtractionConfig | null): Optional extraction configuration applied to all files

**Returns:**

- `Promise<ExtractionResult[]>`: Array of extraction results in the same order as input

**Throws:**

- `Error`: If WASM module is not initialized or any extraction fails

**Example - Process multiple file uploads:**

```typescript title="batch_extract_files.ts"
import { initWasm, batchExtractFiles } from '@kreuzberg/wasm';

await initWasm();

const fileInput = document.getElementById('files') as HTMLInputElement;
const files = Array.from(fileInput.files);

const results = await batchExtractFiles(files, {
  extract_tables: true
});

for (const result of results) {
  console.log(`${result.mimeType}: ${result.content.length} characters`);
}
```

---

## Synchronous Extraction Functions

### extractBytesSync()

Extract content from document bytes synchronously.

**Note:** Synchronous extraction may block the event loop on large documents. Use async extraction (`extractBytes()`) for better performance in most cases.

**Signature:**

```typescript title="TypeScript"
function extractBytesSync(
  data: Uint8Array,
  mimeType: string,
  config?: ExtractionConfig | null
): ExtractionResult
```

**Parameters:**

- `data` (Uint8Array): The document bytes to extract from
- `mimeType` (string): MIME type of the document
- `config` (ExtractionConfig | null): Optional extraction configuration

**Returns:**

- `ExtractionResult`: Extraction result

**Throws:**

- `Error`: If WASM module is not initialized or extraction fails

**Example:**

```typescript title="extract_sync.ts"
import { initWasm, extractBytesSync } from '@kreuzberg/wasm';

await initWasm();

const result = extractBytesSync(pdfBytes, 'application/pdf');
console.log(result.content);
```

---

### batchExtractBytesSync()

Extract content from multiple byte arrays synchronously.

**Signature:**

```typescript title="TypeScript"
function batchExtractBytesSync(
  dataList: Uint8Array[],
  mimeTypes: string[],
  config?: ExtractionConfig | null
): ExtractionResult[]
```

**Parameters:**

- `dataList` (Uint8Array[]): Array of document bytes
- `mimeTypes` (string[]): Array of MIME types
- `config` (ExtractionConfig | null): Optional extraction configuration

**Returns:**

- `ExtractionResult[]`: Array of extraction results

**Throws:**

- `Error`: If WASM module is not initialized or any extraction fails

---

## OCR Functions

### enableOcr()

Enable OCR functionality with the tesseract-wasm backend.

Convenience function that automatically initializes and registers the Tesseract WASM backend for browser environments.

**Signature:**

```typescript title="TypeScript"
async function enableOcr(): Promise<void>
```

**Throws:**

- `Error`: If WASM module is not initialized or not in browser environment

**Requirements:**

- Browser environment with Web Workers support
- Network access to jsDelivr CDN for training data (on first use)
- `createImageBitmap` API support

**Example - Basic OCR:**

```typescript title="enable_ocr.ts"
import { initWasm, enableOcr, extractBytes } from '@kreuzberg/wasm';

async function main() {
  // Initialize WASM module
  await initWasm();

  // Enable OCR with tesseract-wasm
  await enableOcr();

  // Now you can use OCR in extraction
  const imageBytes = new Uint8Array(buffer);
  const result = await extractBytes(imageBytes, 'image/png', {
    ocr: { backend: 'tesseract-wasm', language: 'eng' }
  });

  console.log(result.content); // Extracted text
}

main().catch(console.error);
```

**Example - Multi-language OCR:**

```typescript title="ocr_multilingual.ts"
import { initWasm, enableOcr, extractBytes } from '@kreuzberg/wasm';

await initWasm();
await enableOcr();

// Extract English text
const englishResult = await extractBytes(engImageBytes, 'image/png', {
  ocr: { backend: 'tesseract-wasm', language: 'eng' }
});

// Extract German text - model is cached after first use
const germanResult = await extractBytes(deImageBytes, 'image/png', {
  ocr: { backend: 'tesseract-wasm', language: 'deu' }
});
```

---

## OCR Backend Management

### registerOcrBackend()

Register a custom OCR backend.

**Signature:**

```typescript title="TypeScript"
function registerOcrBackend(backend: OcrBackendProtocol): void
```

**Parameters:**

- `backend` (OcrBackendProtocol): OCR backend implementing the OcrBackendProtocol interface. Required.

**Throws:**

- `Error`: If backend validation fails

**Example:**

```typescript title="register_ocr_backend.ts"
import { registerOcrBackend } from '@kreuzberg/wasm';
import { TesseractWasmBackend } from '@kreuzberg/wasm';

const backend = new TesseractWasmBackend();
await backend.initialize();
registerOcrBackend(backend);
```

---

### getOcrBackend()

Get a registered OCR backend by name.

**Signature:**

```typescript title="TypeScript"
function getOcrBackend(name: string): OcrBackendProtocol | undefined
```

**Parameters:**

- `name` (string): Backend name. Required.

**Returns:**

- `OcrBackendProtocol | undefined`: The OCR backend or undefined if not found

**Example:**

```typescript title="get_ocr_backend.ts"
import { getOcrBackend } from '@kreuzberg/wasm';

const backend = getOcrBackend('tesseract-wasm');
if (backend) {
  console.log('Available languages:', backend.supportedLanguages());
}
```

---

### listOcrBackends()

List all registered OCR backends.

**Signature:**

```typescript title="TypeScript"
function listOcrBackends(): string[]
```

**Returns:**

- `string[]`: Array of registered backend names

**Example:**

```typescript title="list_ocr_backends.ts"
import { listOcrBackends } from '@kreuzberg/wasm';

const backends = listOcrBackends();
console.log('Available OCR backends:', backends);
```

---

### unregisterOcrBackend()

Unregister an OCR backend.

**Signature:**

```typescript title="TypeScript"
async function unregisterOcrBackend(name: string): Promise<void>
```

**Parameters:**

- `name` (string): Backend name to unregister. Required.

**Throws:**

- `Error`: If backend is not found

**Example:**

```typescript title="unregister_ocr_backend.ts"
import { unregisterOcrBackend } from '@kreuzberg/wasm';

await unregisterOcrBackend('tesseract-wasm');
```

---

### clearOcrBackends()

Clear all registered OCR backends and call their shutdown methods.

**Signature:**

```typescript title="TypeScript"
async function clearOcrBackends(): Promise<void>
```

**Example:**

```typescript title="clear_ocr_backends.ts"
import { clearOcrBackends } from '@kreuzberg/wasm';

// Clean up all backends when shutting down
await clearOcrBackends();
```

---

## MIME Type Utilities

### detectMimeFromBytes()

Auto-detect MIME type from file bytes.

**Signature:**

```typescript title="TypeScript"
function detectMimeFromBytes(data: Uint8Array): string
```

**Parameters:**

- `data` (Uint8Array): File bytes to detect MIME type from. Required.

**Returns:**

- `string`: Detected MIME type (e.g., 'application/pdf', 'image/jpeg')

**Example:**

```typescript title="detect_mime.ts"
import { detectMimeFromBytes } from '@kreuzberg/wasm';

const fileBytes = new Uint8Array(buffer);
const mimeType = detectMimeFromBytes(fileBytes);
console.log(`Detected MIME type: ${mimeType}`);
```

---

### getMimeFromExtension()

Get MIME type from file extension.

**Signature:**

```typescript title="TypeScript"
function getMimeFromExtension(extension: string): string | null
```

**Parameters:**

- `extension` (string): File extension (with or without leading dot). Required.

**Returns:**

- `string | null`: MIME type or null if extension is not recognized

**Example:**

```typescript title="get_mime_extension.ts"
import { getMimeFromExtension } from '@kreuzberg/wasm';

const mimeType = getMimeFromExtension('pdf');  // 'application/pdf'
const mimeType2 = getMimeFromExtension('.docx'); // 'application/vnd.openxmlformats-officedocument.wordprocessingml.document'
```

---

### getExtensionsForMime()

Get file extensions for a MIME type.

**Signature:**

```typescript title="TypeScript"
function getExtensionsForMime(mimeType: string): string[]
```

**Parameters:**

- `mimeType` (string): MIME type to look up. Required.

**Returns:**

- `string[]`: Array of file extensions (without leading dots)

**Example:**

```typescript title="get_extensions.ts"
import { getExtensionsForMime } from '@kreuzberg/wasm';

const extensions = getExtensionsForMime('application/pdf');  // ['pdf']
const extensions2 = getExtensionsForMime('image/jpeg');      // ['jpg', 'jpeg']
```

---

### normalizeMimeType()

Normalize MIME type to canonical form.

**Signature:**

```typescript title="TypeScript"
function normalizeMimeType(mimeType: string): string
```

**Parameters:**

- `mimeType` (string): MIME type to normalize. Required.

**Returns:**

- `string`: Normalized MIME type

**Example:**

```typescript title="normalize_mime.ts"
import { normalizeMimeType } from '@kreuzberg/wasm';

const normalized = normalizeMimeType('application/PDF');  // 'application/pdf'
const normalized2 = normalizeMimeType('text/plain');      // 'text/plain'
```

---

## Configuration Loading

### loadConfigFromString()

Load extraction configuration from a string in YAML, JSON, or TOML format.

**Signature:**

```typescript title="TypeScript"
function loadConfigFromString(
  content: string,
  format: 'yaml' | 'toml' | 'json'
): ExtractionConfig
```

**Parameters:**

- `content` (string): Configuration content as a string. Required.
- `format` ('yaml' | 'toml' | 'json'): Configuration format. Required.

**Returns:**

- `ExtractionConfig`: Parsed extraction configuration

**Throws:**

- `Error`: If configuration parsing fails

**Example - YAML configuration:**

```typescript title="load_config_yaml.ts"
import { loadConfigFromString, extractBytes } from '@kreuzberg/wasm';

const yamlConfig = `
extract_tables: true
enable_ocr: true
ocr_config:
  languages: [eng, deu]
`;

const config = loadConfigFromString(yamlConfig, 'yaml');
const result = await extractBytes(data, 'application/pdf', config);
```

**Example - JSON configuration:**

```typescript title="load_config_json.ts"
import { loadConfigFromString } from '@kreuzberg/wasm';

const jsonConfig = '{"extract_tables":true,"enable_ocr":true}';
const config = loadConfigFromString(jsonConfig, 'json');
```

**Example - TOML configuration:**

```typescript title="load_config_toml.ts"
import { loadConfigFromString } from '@kreuzberg/wasm';

const tomlConfig = `
extract_tables = true
enable_ocr = true

[ocr_config]
languages = ["eng", "deu"]
`;

const config = loadConfigFromString(tomlConfig, 'toml');
```

---

## Runtime Detection

### detectRuntime()

Detect the current JavaScript runtime environment.

**Signature:**

```typescript title="TypeScript"
function detectRuntime(): RuntimeType
```

**Returns:**

- `RuntimeType`: One of 'browser', 'node', 'deno', 'bun', or 'unknown'

**Example:**

```typescript title="detect_runtime.ts"
import { detectRuntime } from '@kreuzberg/wasm';

const runtime = detectRuntime();
switch (runtime) {
  case 'browser':
    console.log('Running in browser');
    break;
  case 'node':
    console.log('Running in Node.js');
    break;
  case 'deno':
    console.log('Running in Deno');
    break;
  case 'bun':
    console.log('Running in Bun');
    break;
}
```

---

### getWasmCapabilities()

Get WebAssembly capabilities available in the current runtime.

**Signature:**

```typescript title="TypeScript"
function getWasmCapabilities(): WasmCapabilities
```

**Returns:**

- `WasmCapabilities`: Object containing capability flags:
  - `runtime` (RuntimeType): Detected runtime
  - `hasWasm` (boolean): WebAssembly support
  - `hasWasmStreaming` (boolean): Streaming WASM instantiation
  - `hasFileApi` (boolean): File API (browser)
  - `hasBlob` (boolean): Blob API
  - `hasWorkers` (boolean): Web Worker support
  - `hasSharedArrayBuffer` (boolean): SharedArrayBuffer (restricted)
  - `hasModuleWorkers` (boolean): Module Workers
  - `hasBigInt` (boolean): BigInt support
  - `runtimeVersion` (string | undefined): Runtime version if available

**Example:**

```typescript title="check_capabilities.ts"
import { getWasmCapabilities } from '@kreuzberg/wasm';

const caps = getWasmCapabilities();
console.log(`Runtime: ${caps.runtime}`);
console.log(`WASM: ${caps.hasWasm}`);
console.log(`Workers: ${caps.hasWorkers}`);

if (caps.hasSharedArrayBuffer) {
  console.log('Multi-threading available');
} else {
  console.log('Running in single-threaded mode');
}
```

---

### isBrowser(), isNode(), isDeno(), isBun()

Check if code is running in a specific runtime.

**Signature:**

```typescript title="TypeScript"
function isBrowser(): boolean
function isNode(): boolean
function isDeno(): boolean
function isBun(): boolean
```

**Returns:**

- `boolean`: True if running in the specified runtime

**Example:**

```typescript title="runtime_checks.ts"
import { isBrowser, isNode, extractFile } from '@kreuzberg/wasm';

if (isNode()) {
  // Node.js: use extractFile() for file system access
  const result = await extractFile('./document.pdf');
} else if (isBrowser()) {
  // Browser: use extractFromFile() or extractBytes()
  const result = await extractFromFile(fileInput.files[0]);
}
```

---

### hasWorkers(), hasSharedArrayBuffer()

Check for specific WASM capabilities.

**Signature:**

```typescript title="TypeScript"
function hasWorkers(): boolean
function hasSharedArrayBuffer(): boolean
```

**Returns:**

- `boolean`: True if the capability is available

**Example:**

```typescript title="capability_checks.ts"
import { hasWorkers, hasSharedArrayBuffer } from '@kreuzberg/wasm';

if (hasSharedArrayBuffer()) {
  console.log('Multi-threading with SharedArrayBuffer enabled');
}

if (!hasWorkers()) {
  console.warn('Web Workers not available - some features may be limited');
}
```

---

## Type Adapter Utilities

### fileToUint8Array()

Convert a File or Blob to Uint8Array.

Handles both browser File API and server-side Blob-like objects with a unified interface.

**Signature:**

```typescript title="TypeScript"
async function fileToUint8Array(file: File | Blob): Promise<Uint8Array>
```

**Parameters:**

- `file` (File | Blob): The File or Blob to convert. Required.

**Returns:**

- `Promise<Uint8Array>`: The byte array

**Throws:**

- `Error`: If file cannot be read or exceeds size limit (512 MB)

**Example:**

```typescript title="file_to_bytes.ts"
import { fileToUint8Array, extractBytes } from '@kreuzberg/wasm';

const file = document.getElementById('input').files[0];
const bytes = await fileToUint8Array(file);
const result = await extractBytes(bytes, file.type);
```

---

### configToJS()

Normalize ExtractionConfig for WASM processing.

Converts TypeScript configuration objects to WASM-compatible format, handling null values and nested structures.

**Signature:**

```typescript title="TypeScript"
function configToJS(config: ExtractionConfig | null): Record<string, unknown>
```

**Parameters:**

- `config` (ExtractionConfig | null): The extraction configuration or null

**Returns:**

- `Record<string, unknown>`: Normalized configuration object

**Example:**

```typescript title="config_normalize.ts"
import { configToJS } from '@kreuzberg/wasm/adapters/wasm-adapter';

const config = {
  ocr: { backend: 'tesseract' },
  chunking: { maxChars: 1000 }
};
const wasmConfig = configToJS(config);
```

---

### jsToExtractionResult()

Parse WASM extraction result and convert to TypeScript type.

Handles conversion of WASM-returned objects to proper ExtractionResult types with full validation.

**Signature:**

```typescript title="TypeScript"
function jsToExtractionResult(jsValue: unknown): ExtractionResult
```

**Parameters:**

- `jsValue` (unknown): The raw WASM result value

**Returns:**

- `ExtractionResult`: Properly typed extraction result

**Throws:**

- `Error`: If result structure is invalid

---

### isValidExtractionResult()

Validate that a value conforms to ExtractionResult structure.

Performs structural validation without full type checking.

**Signature:**

```typescript title="TypeScript"
function isValidExtractionResult(value: unknown): value is ExtractionResult
```

**Parameters:**

- `value` (unknown): The value to validate

**Returns:**

- `boolean`: True if value appears to be a valid ExtractionResult

---

## Type Definitions

All types are exported from the `@kreuzberg/wasm` package and shared from `@kreuzberg/core`. Use these types for complete type safety when working with configuration and results.

### Importing Types

```typescript title="TypeScript"
import type {
  ExtractionResult,
  ExtractionConfig,
  OcrConfig,
  ChunkingConfig,
  ImageConfig,
  KeywordsConfig,
  Table,
  ExtractedImage,
  Chunk,
  Metadata,
  OcrBackendProtocol,
  RuntimeType,
  WasmCapabilities
} from '@kreuzberg/wasm';
```

---

## Types

All types are shared via the `@kreuzberg/core` package. Import them for type-safe configuration and results:

```typescript title="TypeScript"
import type {
  ExtractionResult,
  ExtractionConfig,
  OcrConfig,
  ChunkingConfig,
  ImageConfig,
  KeywordsConfig,
  Table,
  ExtractedImage,
  Chunk,
  Metadata,
  OcrBackendProtocol
} from '@kreuzberg/core';
```

### ExtractionResult

The main result object returned from extraction functions.

**Fields:**

- `content` (string): Extracted text content
- `mimeType` (string): MIME type of the document
- `metadata` (Metadata): Document metadata (page count, encoding, etc.)
- `tables` (Table[] | null): Extracted tables (if `extract_tables` enabled)
- `images` (ExtractedImage[] | null): Extracted images (if `extract_images` enabled)
- `chunks` (Chunk[] | null): Text chunks (if `enable_chunking` enabled)
- `detectedLanguages` (string[] | null): Detected language codes (if `enable_language_detection` enabled)

---

### ExtractionConfig

Configuration object for extraction. All fields are optional; defaults are used if not provided.

**Fields:**

- `extract_tables` (boolean): Extract tables as structured data
- `extract_images` (boolean): Extract embedded images
- `extract_metadata` (boolean): Extract document metadata
- `enable_ocr` (boolean): Enable OCR for images and scanned PDFs
- `ocr_config` (OcrConfig): OCR configuration
- `enable_chunking` (boolean): Split text into semantic chunks
- `chunking_config` (ChunkingConfig): Text chunking configuration
- `enable_language_detection` (boolean): Detect document language
- `enable_quality` (boolean): Enable encoding detection and normalization
- `extract_keywords` (boolean): Extract important keywords
- `keywords_config` (KeywordsConfig): Keyword extraction settings

---

### OcrConfig

Configuration for OCR extraction.

**Fields:**

- `backend` (string): OCR backend name (e.g., 'tesseract-wasm')
- `language` (string): Language code for OCR (e.g., 'eng', 'deu', 'fra')
- `languages` (string[]): Multiple languages for OCR
- `dpi` (number): DPI for OCR processing
- `preprocessing` (OcrPreprocessing): Image preprocessing settings

---

### ChunkingConfig

Configuration for text chunking.

**Fields:**

- `maxChars` (number): Maximum characters per chunk
- `maxTokens` (number): Maximum tokens per chunk
- `chunkOverlap` (number): Overlap between chunks in characters/tokens

---

### ImageConfig

Configuration for image extraction.

**Fields:**

- `extractImages` (boolean): Extract images from documents
- `targetDpi` (number): Target DPI for extracted images
- `maxImageDimension` (number): Maximum pixel dimension for images

---

### KeywordsConfig

Configuration for keyword extraction.

**Fields:**

- `maxKeywords` (number): Maximum number of keywords to extract
- `method` (string): Keyword extraction method (e.g., 'yake')

---

### Table

Extracted table structure.

**Fields:**

- `cells` (string[][]): 2D array of table cells
- `markdown` (string): Table in Markdown format
- `pageNumber` (number): Page number where table appears

---

### ExtractedImage

Image extracted from document.

**Fields:**

- `data` (Uint8Array): Image bytes
- `format` (string): Image format (e.g., 'png', 'jpeg')
- `imageIndex` (number): Index within document
- `pageNumber` (number | null): Page number (if applicable)
- `width` (number | null): Image width in pixels
- `height` (number | null): Image height in pixels
- `colorspace` (string | null): Color space (e.g., 'RGB', 'CMYK')
- `bitsPerComponent` (number | null): Bits per color component
- `isMask` (boolean): Whether this is a mask image
- `description` (string | null): Image description if available

---

### Chunk

Text chunk from chunking operation.

**Fields:**

- `content` (string): Chunk text content
- `metadata` (ChunkMetadata): Metadata about the chunk
- `embedding` (number[] | null): Vector embedding (if available)

**ChunkMetadata:**

- `charStart` (number): Starting character position
- `charEnd` (number): Ending character position
- `chunkIndex` (number): Index of this chunk
- `totalChunks` (number): Total number of chunks
- `tokenCount` (number | null): Token count if available

---

### Metadata

Document metadata.

**Fields:**

- `pageCount` (number | null): Number of pages (if applicable)
- `encoding` (string | null): Text encoding
- `format` (string): Document format
- `author` (string | null): Document author
- `title` (string | null): Document title
- `createdAt` (string | null): Creation timestamp
- `modifiedAt` (string | null): Last modification timestamp
- [Additional format-specific fields]

---

## Platform-Specific Notes

### Browser

**Requirements:**

- Modern browser with WebAssembly support (Chrome 91+, Firefox 90+, Safari 16.4+)
- File API for file uploads

**SharedArrayBuffer for Multi-Threading:**

To enable multi-threaded extraction, set these HTTP headers:

```
Cross-Origin-Opener-Policy: same-origin
Cross-Origin-Embedder-Policy: require-corp
```

**Example with Express.js:**

```typescript title="express_sab_headers.ts"
import express from 'express';

const app = express();

app.use((req, res, next) => {
  res.setHeader('Cross-Origin-Opener-Policy', 'same-origin');
  res.setHeader('Cross-Origin-Embedder-Policy', 'require-corp');
  next();
});
```

**Example with Vite:**

```typescript title="vite.config.ts"
import { defineConfig } from 'vite';

export default defineConfig({
  server: {
    headers: {
      'Cross-Origin-Opener-Policy': 'same-origin',
      'Cross-Origin-Embedder-Policy': 'require-corp'
    }
  }
});
```

---

### Node.js

**Requirements:**

- Node.js 18 or higher
- WASM support (available by default)

**Example:**

```typescript title="nodejs_extraction.ts"
import { extractFile, initWasm } from '@kreuzberg/wasm';

async function main() {
  await initWasm();
  const result = await extractFile('./document.pdf');
  console.log(result.content);
}

main().catch(console.error);
```

---

### Deno

**Requirements:**

- Deno 1.0 or higher
- Read permissions for files (`--allow-read`)
- Network permissions for OCR training data (`--allow-net`)

**Import:**

```typescript title="deno_import.ts"
import { extractFile, initWasm } from "npm:@kreuzberg/wasm@^4.0.0";

// Must run with: deno run --allow-read --allow-net script.ts
```

**Example:**

```typescript title="deno_example.ts"
import { extractFile, initWasm } from "npm:@kreuzberg/wasm@^4.0.0";

async function main() {
  await initWasm();
  const result = await extractFile('./document.pdf');
  console.log(result.content);
}

main().catch(console.error);
```

---

### Bun

**Requirements:**

- Bun 1.x or higher
- WASM support (available by default)

**Example:**

```typescript title="bun_example.ts"
import { extractFile, initWasm } from '@kreuzberg/wasm';

async function main() {
  await initWasm();
  const result = await extractFile('./document.pdf');
  console.log(result.content);
}

main().catch(console.error);
```

---

### Cloudflare Workers

**Requirements:**

- Cloudflare Workers runtime
- Bundle size considerations (10MB limit compressed)

**HTTP Headers:**

Cloudflare Workers automatically handle necessary CORS headers. For multi-threading, ensure:

```typescript title="cloudflare_worker.ts"
export default {
  async fetch(request: Request): Promise<Response> {
    const response = new Response(body);
    response.headers.set('Cross-Origin-Opener-Policy', 'same-origin');
    response.headers.set('Cross-Origin-Embedder-Policy', 'require-corp');
    return response;
  }
};
```

**Memory Constraints:**

For large documents, use chunking to reduce memory usage:

```typescript title="cloudflare_memory_efficient.ts"
import { extractBytes } from '@kreuzberg/wasm';

export default {
  async fetch(request: Request): Promise<Response> {
    const formData = await request.formData();
    const file = formData.get('file') as File;
    const arrayBuffer = await file.arrayBuffer();
    const bytes = new Uint8Array(arrayBuffer);

    const result = await extractBytes(bytes, file.type, {
      chunking_config: { maxChars: 1000 }
    });

    return Response.json({
      text: result.content,
      metadata: result.metadata
    });
  }
};
```

---

## Common Patterns

### Pattern: Runtime-Aware File Loading

Automatically select the appropriate extraction function based on runtime:

```typescript title="runtime_aware_loading.ts"
import {
  extractFile,
  extractFromFile,
  isNode,
  isBrowser,
  initWasm
} from '@kreuzberg/wasm';

await initWasm();

async function extractAny(input: string | File): Promise<ExtractionResult> {
  if (isNode() && typeof input === 'string') {
    return await extractFile(input);
  } else if (isBrowser() && input instanceof File) {
    return await extractFromFile(input);
  } else {
    throw new Error('Invalid input for current runtime');
  }
}
```

---

### Pattern: Graceful OCR Initialization

Initialize OCR with fallback to text-only extraction:

```typescript title="ocr_graceful_init.ts"
import { initWasm, enableOcr, extractBytes } from '@kreuzberg/wasm';

async function extractWithOcrFallback(bytes: Uint8Array, mimeType: string) {
  await initWasm();

  let config = {};
  try {
    await enableOcr();
    config = { ocr: { backend: 'tesseract-wasm', language: 'eng' } };
  } catch (error) {
    console.warn('OCR unavailable, continuing with text extraction', error);
  }

  return await extractBytes(bytes, mimeType, config);
}
```

---

### Pattern: Batch Processing with Progress

Extract multiple files with progress tracking:

```typescript title="batch_with_progress.ts"
import { initWasm, batchExtractBytes } from '@kreuzberg/wasm';

async function extractWithProgress(
  files: File[],
  onProgress: (current: number, total: number) => void
) {
  await initWasm();

  const results = [];
  for (let i = 0; i < files.length; i++) {
    const fileBytes = await files[i].arrayBuffer();
    const result = await extractBytes(
      new Uint8Array(fileBytes),
      files[i].type
    );
    results.push(result);
    onProgress(i + 1, files.length);
  }

  return results;
}
```

---

### Pattern: Configuration Management

Load configuration from environment or file:

```typescript title="config_management.ts"
import { loadConfigFromString, extractBytes } from '@kreuzberg/wasm';

async function extractWithConfig(bytes: Uint8Array, mimeType: string) {
  let config = null;

  // Try to load from environment variable
  const configStr = process.env.KREUZBERG_CONFIG;
  if (configStr) {
    try {
      config = loadConfigFromString(configStr, 'json');
    } catch (error) {
      console.warn('Failed to parse config from environment:', error);
    }
  }

  // Default config if not loaded
  if (!config) {
    config = {
      extract_tables: true,
      extract_metadata: true
    };
  }

  return await extractBytes(bytes, mimeType, config);
}
```

---

## Supported Formats

| Category | Formats |
|----------|---------|
| **Documents** | PDF, DOCX, DOC, PPTX, PPT, XLSX, XLS, ODT, ODP, ODS, RTF |
| **Images** | PNG, JPEG, JPG, WEBP, BMP, TIFF, GIF |
| **Web** | HTML, XHTML, XML, EPUB |
| **Text** | TXT, MD, RST, LaTeX, CSV, TSV, JSON, YAML, TOML, ORG, BIB, TeX, FB2 |
| **Email** | EML, MSG |
| **Archives** | ZIP, TAR, 7Z |
| **Other** | And 30+ more formats |

---

## Supported MIME Types

Common MIME types supported by Kreuzberg WASM:

### Documents

- `application/pdf` - PDF documents
- `application/vnd.openxmlformats-officedocument.wordprocessingml.document` - DOCX (Word)
- `application/msword` - DOC (Word 97-2003)
- `application/vnd.openxmlformats-officedocument.presentationml.presentation` - PPTX (PowerPoint)
- `application/vnd.ms-powerpoint` - PPT (PowerPoint 97-2003)
- `application/vnd.openxmlformats-officedocument.spreadsheetml.sheet` - XLSX (Excel)
- `application/vnd.ms-excel` - XLS (Excel 97-2003)
- `application/vnd.oasis.opendocument.text` - ODT (OpenDocument Text)
- `application/vnd.oasis.opendocument.presentation` - ODP (OpenDocument Presentation)
- `application/vnd.oasis.opendocument.spreadsheet` - ODS (OpenDocument Spreadsheet)
- `text/rtf` - RTF (Rich Text Format)

### Images

- `image/png` - PNG
- `image/jpeg` - JPEG
- `image/webp` - WebP
- `image/bmp` - BMP
- `image/tiff` - TIFF
- `image/gif` - GIF

### Text

- `text/plain` - Plain text
- `text/markdown` - Markdown
- `text/html` - HTML
- `application/json` - JSON
- `text/xml` - XML
- `application/xml` - XML (alternative)
- `text/yaml` - YAML
- `text/csv` - CSV
- `text/tab-separated-values` - TSV

### Archives

- `application/zip` - ZIP
- `application/x-tar` - TAR
- `application/x-7z-compressed` - 7Z

---

## Platform Support Matrix

| Function | Browser | Node.js | Deno | Bun | Workers |
|----------|---------|---------|------|-----|---------|
| `initWasm()` | Yes | Yes | Yes | Yes | Yes |
| `extractBytes()` | Yes | Yes | Yes | Yes | Yes |
| `extractFile()` | No | Yes | Yes | Yes | No |
| `extractFromFile()` | Yes | No | No | No | No |
| `enableOcr()` | Yes | No | No | No | No |
| `initThreadPool()` | Yes | No | No | No | No |
| `batchExtractFiles()` | Yes | No | No | No | No |

**Note:** OCR is only available in browser environments with Web Worker support due to dependency on tesseract-wasm and browser APIs like `createImageBitmap`.

---

## Troubleshooting

### "WASM module failed to initialize"

Ensure your bundler is configured to handle WASM files:

**Vite:**

```typescript title="vite.config.ts"
export default {
  optimizeDeps: {
    exclude: ['@kreuzberg/wasm']
  }
}
```

**Webpack:**

```javascript title="webpack.config.js"
module.exports = {
  experiments: {
    asyncWebAssembly: true
  }
}
```

---

### "Module not found: @kreuzberg/core"

The `@kreuzberg/core` package is a peer dependency. Install it:

```bash
npm install @kreuzberg/core
```

---

### "SharedArrayBuffer is not available"

This is expected in some browsers or when headers are not set. Multi-threading will not be available, but extraction will continue in single-threaded mode.

To enable multi-threading, set the required HTTP headers (see Platform-Specific Notes > Browser).

---

### Memory Issues in Cloudflare Workers

For large documents, process in smaller chunks:

```typescript title="cloudflare_chunked.ts"
const result = await extractBytes(pdfBytes, 'application/pdf', {
  chunking_config: { maxChars: 1000 }
});
```

---

### WASM Module Not Loading

**Symptoms:** "Failed to load WASM module" error on initialization

**Causes:**
- Network issues preventing WASM download
- Bundler misconfiguration (not handling .wasm files correctly)
- CORS restrictions blocking module fetch
- Module not included in bundle

**Solutions:**
1. Check browser network tab for failed requests
2. Configure bundler (see "WASM module failed to initialize" section)
3. Ensure CORS headers allow WASM requests
4. Use CDN-delivered version as fallback

---

### SharedArrayBuffer Not Available

**Symptoms:** Multi-threading features disabled, or "SharedArrayBuffer is not available" warning

**Causes:**
- HTTPS context not used (required for security)
- Missing Cross-Origin-Opener-Policy (COOP) headers
- Missing Cross-Origin-Embedder-Policy (COEP) headers
- Old browser version without SharedArrayBuffer support

**Solutions:**
1. Ensure application runs over HTTPS in production
2. Set required headers (see Platform-Specific Notes > Browser section):
   - `Cross-Origin-Opener-Policy: same-origin`
   - `Cross-Origin-Embedder-Policy: require-corp`
3. Update browser to latest version
4. Application will automatically fall back to single-threaded mode

---

### OCR Not Available or Not Working

**Symptoms:** "OCR is only available in browser" error or OCR produces no output

**Causes:**
- Attempting to use `enableOcr()` outside of browser environment (Node.js/Deno/Workers)
- Web Workers not supported or blocked
- Training data not loading from jsDelivr CDN
- Language model not available for selected language

**Solutions:**
1. Check runtime with `isBrowser()` before enabling OCR:
   ```typescript title="check_browser.ts"
   import { isBrowser, enableOcr } from '@kreuzberg/wasm';

   if (isBrowser()) {
     await enableOcr();
   }
   ```

2. Verify Web Worker support:
   ```typescript title="check_workers.ts"
   import { hasWorkers } from '@kreuzberg/wasm';

   if (hasWorkers()) {
     console.log('Web Workers available');
   }
   ```

3. Check supported languages:
   ```typescript title="check_ocr_languages.ts"
   import { getOcrBackend } from '@kreuzberg/wasm';

   const backend = getOcrBackend('tesseract-wasm');
   if (backend) {
     const langs = backend.supportedLanguages();
     console.log('Supported languages:', langs);
     // Verify your language is in the list
   }
   ```

4. Ensure network access to jsDelivr CDN:
   - First OCR call downloads training data (~50MB for English)
   - Subsequent calls use cached data
   - May fail without internet connection

5. Handle initialization errors gracefully:
   ```typescript title="ocr_graceful.ts"
   import { enableOcr, extractBytes } from '@kreuzberg/wasm';

   let ocrEnabled = false;
   try {
     await enableOcr();
     ocrEnabled = true;
   } catch (error) {
     console.warn('OCR initialization failed:', error);
   }

   const config = ocrEnabled
     ? { ocr: { backend: 'tesseract-wasm', language: 'eng' } }
     : {};

   const result = await extractBytes(bytes, 'application/pdf', config);
   ```

---

### WASM Module Size and Performance

**Symptoms:** Large bundle size or slow initial load

**Context:**
- WASM module: ~5MB uncompressed
- Gzip compressed: ~1.5-2MB
- OCR training data (per language): ~20-50MB (downloaded on demand, cached)

**Optimization strategies:**
1. Use code splitting to load WASM only when needed
2. Compress with gzip/brotli (bundlers do this automatically)
3. Load training data selectively (only load languages you need)
4. Use `extractBytes()` for in-memory processing to avoid file I/O
5. For large documents, enable chunking to reduce memory usage

---

## Multi-Threading with wasm-bindgen-rayon

Kreuzberg WASM leverages [wasm-bindgen-rayon](https://docs.rs/wasm-bindgen-rayon/) to enable multi-threaded document processing with SharedArrayBuffer support.

### Initializing Thread Pool

Initialize the thread pool with available CPU cores:

```typescript title="init_thread_pool.ts"
import { initThreadPool } from '@kreuzberg/wasm';

// Initialize thread pool for multi-threaded extraction
await initThreadPool(navigator.hardwareConcurrency);

// Now extractions will use multiple threads for better performance
const result = await extractBytes(pdfBytes, 'application/pdf');
```

### Graceful Degradation

The library handles thread pool initialization gracefully:

```typescript title="thread_pool_graceful.ts"
import { initThreadPool } from '@kreuzberg/wasm';

try {
  await initThreadPool(navigator.hardwareConcurrency);
  console.log('Multi-threading enabled');
} catch (error) {
  // Fall back to single-threaded processing
  console.warn('Multi-threading unavailable:', error);
  console.log('Using single-threaded extraction');
}

// Extraction will work in both cases
const result = await extractBytes(pdfBytes, 'application/pdf');
```

---

## See Also

- [Configuration Reference](/docs/reference/configuration.md)
- [Type Reference](/docs/reference/types.md)
- [Error Handling](/docs/reference/errors.md)
- [Getting Started Guide](/docs/getting-started/wasm.md)
- [WASM Examples](/docs/guides/examples.md#wasm)
