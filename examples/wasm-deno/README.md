# Kreuzberg WASM Deno Examples

Complete example applications demonstrating document extraction using `@kreuzberg/wasm` with Deno runtime.

## Prerequisites

- Deno 1.x or 2.x ([install here](https://deno.land))
- NPM packages available through deno.land (or configured with deno.json imports)

## Running Examples

### Basic Extraction

Extract text from documents with different configurations:

```bash
deno task basic
# or
deno run --allow-read basic.ts
```

Demonstrates:
- `extractBytes()` - async extraction from byte arrays
- `extractBytesSync()` - synchronous extraction from byte arrays
- Configuration options (quality, chunking, caching)
- MIME type detection
- Error handling

### Batch Processing

Process multiple documents efficiently:

```bash
deno task batch
# or
deno run --allow-read batch.ts
```

Demonstrates:
- Parallel batch processing
- Error handling for mixed results
- Performance metrics
- Combining multiple document types

### OCR Extraction

Extract text from scanned documents and images using Tesseract OCR:

```bash
deno task ocr
# or
deno run --allow-read ocr.ts
```

**Note:** OCR functionality requires Tesseract backend to be available in your environment.

Demonstrates:
- OCR with language selection (English, German, etc.)
- Force OCR on text-based PDFs
- Configuration with Tesseract options
- Metadata extraction (detected language, confidence, etc.)

## Example Features

### Basic Example (basic.ts)

- Synchronous and asynchronous extraction
- Configuration with quality processing
- Metadata inspection (MIME type, content length)
- Error handling with try-catch

### Batch Example (batch.ts)

- Sequential batch processing
- Parallel async processing
- Combining results
- Error handling for individual failures
- Progress tracking

### OCR Example (ocr.ts)

- Language-specific OCR (eng, deu, fra, etc.)
- Force OCR flag for mixed documents
- Tesseract configuration (PSM modes)
- OCR metadata inspection
- Skipping tests when dependencies unavailable

## Configuration Options

### ExtractionConfig

```typescript
interface ExtractionConfig {
  chunking?: ChunkingConfig;          // Text chunking settings
  ocr?: OcrConfig;                    // OCR settings
  forceOcr?: boolean;                 // Force OCR even for text PDFs
  pdf?: PdfConfig;                    // PDF-specific options
  imageExtraction?: ImageExtractionConfig;
  languageDetection?: LanguageDetectionConfig;
  postProcessor?: PostProcessorConfig;
  tokenReduction?: TokenReductionConfig;
}
```

### OcrConfig

```typescript
interface OcrConfig {
  backend: string;                    // "tesseract"
  language?: string;                  // ISO 639-3 language code (e.g., "eng", "deu")
  tesseractConfig?: TesseractConfig;  // Tesseract-specific options
}
```

### ChunkingConfig

```typescript
interface ChunkingConfig {
  maxChars?: number;                  // Maximum characters per chunk
  maxOverlap?: number;                // Overlap between chunks
}
```

## Document Types Supported

- **PDF**: application/pdf
- **Microsoft Word**: application/vnd.openxmlformats-officedocument.wordprocessingml.document
- **Microsoft Excel**: application/vnd.openxmlformats-officedocument.spreadsheetml.sheet
- **Microsoft PowerPoint**: application/vnd.openxmlformats-officedocument.presentationml.presentation
- **Images**: image/png, image/jpeg, image/webp, etc.
- **HTML**: text/html
- **Plain Text**: text/plain
- **JSON**: application/json

## Test Fixtures

Sample documents are expected in the `fixtures/` directory. The examples include fallback handling for missing fixtures.

```
examples/wasm-deno/
├── fixtures/
│   └── sample.pdf     # Example PDF document
```

You can copy test documents:

```bash
cp /path/to/test_documents/pdfs/fake_memo.pdf fixtures/sample.pdf
```

## API Reference

### extractBytes()

```typescript
async function extractBytes(
  data: Uint8Array,
  mimeType: string,
  config?: ExtractionConfig
): Promise<ExtractionResult>
```

Asynchronously extract text from byte array with optional configuration.

### extractBytesSync()

```typescript
function extractBytesSync(
  data: Uint8Array,
  mimeType: string,
  config?: ExtractionConfig
): ExtractionResult
```

Synchronously extract text from byte array. Use when async is not available.

## ExtractionResult

```typescript
interface ExtractionResult {
  content: string;                    // Extracted text
  mimeType: string;                   // Detected MIME type
  metadata: Record<string, unknown>;  // Format-specific metadata
  tables?: Table[];                   // Extracted tables
  images?: ExtractedImage[];          // Extracted images (if enabled)
}
```

## Error Handling

Examples demonstrate proper error handling patterns:

```typescript
try {
  const result = await extractBytes(data, "application/pdf");
  console.log(result.content);
} catch (error) {
  if (error instanceof Error) {
    console.error(`Extraction failed: ${error.message}`);
  } else {
    console.error("Unknown error:", error);
  }
}
```

## Performance Tips

1. **Use async extraction** for better performance with multiple documents
2. **Batch processing** is optimized for parallel operations
3. **Disable OCR** if not needed (significant performance impact)
4. **Configure chunking** appropriately for your use case
5. **Cache results** when processing the same documents repeatedly

## Troubleshooting

### Module not found errors

Ensure you're running with proper import resolution:

```bash
deno run --allow-read --import-map=deno.json basic.ts
```

### OCR not available

OCR examples will gracefully skip if Tesseract is unavailable. Install:

```bash
# macOS (via Homebrew)
brew install tesseract

# Ubuntu/Debian
sudo apt-get install tesseract-ocr

# Windows (via Chocolatey)
choco install tesseract
```

### Permission errors

Ensure you include `--allow-read` permission:

```bash
deno run --allow-read basic.ts
```

## Further Reading

- [Kreuzberg Documentation](https://kreuzberg.dev)
- [Deno Manual](https://docs.deno.com)
- [WebAssembly Guide](https://developer.mozilla.org/en-US/docs/WebAssembly/)
