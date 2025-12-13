# WASM Code Snippets Index

Complete reference of TypeScript code snippets for Kreuzberg WASM bindings.

## Getting Started (6 files)

Essential patterns for initializing and using Kreuzberg WASM.

- `basic-extract.ts` - Fundamental extractBytes usage with PDF documents
- `browser-file-input.ts` - Handle file input in browser environments
- `async-extraction.ts` - Parallel extraction with Promise.all
- `initialization.ts` - WASM module initialization with capability detection
- `runtime-detection.ts` - Detect and adapt to runtime environment (browser/Node/Deno/Bun)
- `batch-processing.ts` - Process multiple documents with concurrency control

## API (8 files)

Core extraction functions and result handling.

- `extract-bytes.ts` - Extract from Uint8Array with basic configuration
- `extract-file.ts` - File system extraction for Node.js, Deno, and Bun
- `extract-from-file.ts` - Browser-friendly File/Blob extraction
- `chunked-extraction.ts` - Split documents into semantic chunks
- `image-extraction.ts` - Extract and display images from documents
- `table-extraction.ts` - Parse table structures and markdown
- `metadata-extraction.ts` - Access document metadata fields

## Configuration (7 files)

Configuration patterns for different extraction scenarios.

- `basic-config.ts` - Minimal configuration with common options
- `ocr-config.ts` - OCR-specific settings and language selection
- `chunking-config.ts` - Chunking parameters for text segmentation
- `image-config.ts` - Image extraction quality and size settings
- `combined-config.ts` - Full configuration using all available options
- `conditional-config.ts` - Adaptive configuration based on file size

## OCR (5 files)

Optical Character Recognition patterns with Tesseract WASM.

- `enable-ocr.ts` - Initialize and register Tesseract WASM backend
- `multi-language-ocr.ts` - Extract text in multiple languages
- `progress-tracking.ts` - Monitor OCR progress with callbacks
- `ocr-backend-registration.ts` - Register, list, and unregister OCR backends
- `ocr-error-handling.ts` - Error handling and fallback strategies

## Metadata (4 files)

Extract and process document metadata.

- `extract-metadata.ts` - Access document properties (title, author, dates)
- `filter-metadata.ts` - Create document summaries and comparisons
- `metadata-with-chunks.ts` - Combine chunk metadata with document metadata
- `image-metadata.ts` - Extract detailed image properties

## Advanced (6 files)

Production patterns and complex scenarios.

- `parallel-extraction.ts` - Parallel processing with Web Workers
- `error-recovery.ts` - Retry logic with exponential backoff
- `memory-management.ts` - Batch processing for large document sets
- `streaming-extraction.ts` - Process documents from streaming responses
- `worker-extraction.ts` - Web Worker pool implementation
- `custom-pipeline.ts` - Create custom processing pipelines

## Cache (3 files)

Caching strategies for performance optimization.

- `ocr-cache.ts` - Cache Tesseract WASM models in memory
- `result-caching.ts` - Cache extraction results using SHA-256 hashing
- `session-storage.ts` - Persist results in browser SessionStorage

## Utils (4 files)

Utility functions and helpers.

- `mime-detection.ts` - Detect MIME type from file magic bytes
- `config-validation.ts` - Validate configuration objects
- `type-guards.ts` - TypeScript type predicates for result validation
- `file-conversion.ts` - Convert between File, Blob, and Uint8Array

## Plugins (5 files)

Custom plugin patterns and plugin system management.

- `post-processor-custom.ts` - Implement custom post-processors
- `validator-custom.ts` - Create content validators
- `ocr-backend-custom.ts` - Implement custom OCR backends
- `plugin-lifecycle.ts` - Manage plugin initialization and cleanup
- `plugin-pipeline.ts` - Chain plugins in processing pipeline

## Usage

Reference snippets by their category and filename:

```typescript
--8<-- "docs/snippets/wasm/getting-started/basic-extract.ts"
```

## API Reference

All snippets are based on the following exports from `@kreuzberg/wasm`:

### Core Functions
- `initWasm()` - Initialize WASM module
- `extractBytes(data, mimeType, config?)` - Extract from bytes
- `extractFile(path, mimeType?, config?)` - Extract from file (Node/Deno/Bun)
- `extractFromFile(file, mimeType?, config?)` - Extract from File/Blob (Browser)
- `enableOcr()` - Enable Tesseract WASM OCR

### Types
- `ExtractionConfig` - Configuration object
- `ExtractionResult` - Extraction result
- `Chunk` - Chunked content unit
- `Table` - Table data
- `ExtractedImage` - Image data from document

### Runtime Detection
- `detectRuntime()` - Get current runtime type
- `isBrowser()` / `isNode()` / `isDeno()` / `isBun()` - Runtime checks
- `getWasmCapabilities()` - Check available features

## File Statistics

- Total Snippets: 46
- Categories: 9
- Lines of Code: ~1,500
- Average per Snippet: 25-35 lines

## Best Practices

1. Always call `initWasm()` before using extraction functions
2. Use `try-catch` blocks for error handling
3. Leverage runtime detection for cross-platform code
4. Use configuration to optimize for your use case
5. Implement caching for repeated extractions
6. Use Web Workers for parallel processing in browsers

## See Also

- Main documentation: `docs/index.md`
- WASM examples: `docs/examples/wasm/`
- TypeScript binding source: `crates/kreuzberg-wasm/typescript/`
