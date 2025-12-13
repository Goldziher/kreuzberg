# Kreuzberg

[![Rust](https://img.shields.io/crates/v/kreuzberg?label=Rust)](https://crates.io/crates/kreuzberg)
[![Python](https://img.shields.io/pypi/v/kreuzberg?label=Python)](https://pypi.org/project/kreuzberg/)
[![TypeScript](https://img.shields.io/npm/v/@kreuzberg/node?label=TypeScript)](https://www.npmjs.com/package/@kreuzberg/node)
[![WASM](https://img.shields.io/npm/v/@kreuzberg/wasm?label=WASM)](https://www.npmjs.com/package/@kreuzberg/wasm)
[![Ruby](https://img.shields.io/gem/v/kreuzberg?label=Ruby)](https://rubygems.org/gems/kreuzberg)
[![Java](https://img.shields.io/maven-central/v/dev.kreuzberg/kreuzberg?label=Java)](https://central.sonatype.com/artifact/dev.kreuzberg/kreuzberg)
[![Go](https://img.shields.io/github/v/tag/kreuzberg-dev/kreuzberg?label=Go)](https://pkg.go.dev/github.com/kreuzberg-dev/kreuzberg)
[![C#](https://img.shields.io/nuget/v/Goldziher.Kreuzberg?label=C%23)](https://www.nuget.org/packages/Goldziher.Kreuzberg/)

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Documentation](https://img.shields.io/badge/docs-kreuzberg.dev-blue)](https://kreuzberg.dev/)
[![Discord](https://img.shields.io/badge/Discord-Join%20our%20community-7289da)](https://discord.gg/pXxagNK2zN)

High-performance document intelligence for browsers, Deno, and Cloudflare Workers, powered by WebAssembly.

Extract text, tables, images, and metadata from 50+ file formats including PDF, DOCX, PPTX, XLSX, images, and more.

> **🚀 Version 4.0.0 Release Candidate**
> This is a pre-release version. We invite you to test the library and [report any issues](https://github.com/kreuzberg-dev/kreuzberg/issues) you encounter.

## Features

- **50+ File Formats**: PDF, DOCX, PPTX, XLSX, images, HTML, Markdown, XML, JSON, and more
- **OCR Support**: Built-in tesseract-wasm with 40+ languages for scanned documents
- **Table Extraction**: Advanced table detection and structured data extraction
- **Multi-Runtime**: Browser, Node.js, Deno, and Cloudflare Workers support
- **Type-Safe**: Full TypeScript definitions from shared @kreuzberg/core package
- **100% API Parity**: All extraction functions from the Node.js binding
- **Plugin System**: Custom post-processors, validators, and OCR backends
- **Optimized Bundle**: <5MB uncompressed, <2MB compressed
- **Zero Configuration**: Works out of the box with sensible defaults

## Requirements

- **Browser**: Modern browsers with WebAssembly support (Chrome 91+, Firefox 90+, Safari 16.4+)
- **Node.js**: 18 or higher
- **Deno**: 1.0 or higher
- **Cloudflare Workers**: Compatible with Workers runtime

### Optional Dependencies

- **tesseract-wasm**: Automatically loaded for OCR functionality (40+ language support)

## Installation

```bash
npm install @kreuzberg/wasm
```

Or with pnpm:

```bash
pnpm add @kreuzberg/wasm
```

Or with yarn:

```bash
yarn add @kreuzberg/wasm
```

### Deno

```typescript
import { extractBytes } from "npm:@kreuzberg/wasm@^4.0.0";
```

## Quick Start

### Browser (ESM)

```typescript
import { extractFile } from '@kreuzberg/wasm';

async function handleFileUpload() {
  const fileInput = document.querySelector<HTMLInputElement>('#file-upload');
  const file = fileInput.files[0];

  const result = await extractFile(file, {
    extract_tables: true,
    extract_images: true
  });

  console.log('Extracted text:', result.content);
  console.log('Tables found:', result.tables.length);
}
```

### Node.js (ESM)

```typescript
import { extractBytes } from '@kreuzberg/wasm';
import { readFile } from 'fs/promises';

const pdfBytes = await readFile('./document.pdf');
const result = await extractBytes(
  new Uint8Array(pdfBytes),
  'application/pdf',
  { extract_tables: true }
);

console.log(result.content);
console.log('Found', result.tables.length, 'tables');
```

### Deno

```typescript
import { extractBytes } from "npm:@kreuzberg/wasm@^4.0.0";

const pdfBytes = await Deno.readFile("./document.pdf");
const result = await extractBytes(pdfBytes, "application/pdf");

console.log(result.content);
```

### Cloudflare Workers

```typescript
import { extractBytes } from '@kreuzberg/wasm';

export default {
  async fetch(request: Request): Promise<Response> {
    if (request.method === 'POST') {
      const formData = await request.formData();
      const file = formData.get('file') as File;

      const arrayBuffer = await file.arrayBuffer();
      const bytes = new Uint8Array(arrayBuffer);

      const result = await extractBytes(bytes, file.type);

      return Response.json({
        text: result.content,
        metadata: result.metadata,
        tables: result.tables
      });
    }

    return new Response('Upload a file', { status: 400 });
  }
};
```

## Examples

Kreuzberg WASM includes complete working examples for different environments:

- **[Deno](../../examples/wasm-deno)** - Server-side document extraction with Deno runtime. Demonstrates basic extraction, batch processing, and OCR capabilities.
- **[Cloudflare Workers](../../examples/wasm-cloudflare-workers)** - Serverless API for document processing on the edge. Includes file upload endpoint, error handling, and production-ready configuration.
- **[Browser](../../examples/wasm-browser)** - Interactive web application with drag-and-drop file upload, progress tracking, and multi-threaded extraction using Vite.

See the [examples documentation](../../examples/wasm/README.md) for a comprehensive overview and comparison of all examples.

## Multi-Threading with wasm-bindgen-rayon

Kreuzberg WASM leverages [wasm-bindgen-rayon](https://docs.rs/wasm-bindgen-rayon/latest/wasm_bindgen_rayon/) to enable multi-threaded document processing in browsers and server environments with SharedArrayBuffer support.

### Initializing the Thread Pool

To unlock multi-threaded performance, initialize the thread pool with the available CPU cores:

```typescript
import { initThreadPool } from '@kreuzberg/wasm';

// Initialize thread pool for multi-threaded extraction
await initThreadPool(navigator.hardwareConcurrency);

// Now extractions will use multiple threads for better performance
const result = await extractBytes(pdfBytes, 'application/pdf');
```

### Required HTTP Headers for SharedArrayBuffer

Multi-threading requires specific HTTP headers to enable SharedArrayBuffer in browsers:

**Important:** These headers are required for the thread pool to function. Without them, the library will fall back to single-threaded processing.

Set these headers in your server configuration:

```
Cross-Origin-Opener-Policy: same-origin
Cross-Origin-Embedder-Policy: require-corp
```

#### Server Configuration Examples

**Express.js:**
```javascript
app.use((req, res, next) => {
  res.setHeader('Cross-Origin-Opener-Policy', 'same-origin');
  res.setHeader('Cross-Origin-Embedder-Policy', 'require-corp');
  next();
});
```

**Nginx:**
```nginx
add_header 'Cross-Origin-Opener-Policy' 'same-origin';
add_header 'Cross-Origin-Embedder-Policy' 'require-corp';
```

**Apache:**
```apache
Header set Cross-Origin-Opener-Policy "same-origin"
Header set Cross-Origin-Embedder-Policy "require-corp"
```

**Cloudflare Workers:**
```javascript
export default {
  async fetch(request: Request): Promise<Response> {
    const response = new Response(body);
    response.headers.set('Cross-Origin-Opener-Policy', 'same-origin');
    response.headers.set('Cross-Origin-Embedder-Policy', 'require-corp');
    return response;
  }
};
```

### Browser Compatibility

Multi-threading with SharedArrayBuffer is available in:

- **Chrome/Edge**: 74+
- **Firefox**: 79+
- **Safari**: 15.2+
- **Opera**: 60+

In unsupported browsers or when headers are not set, the library automatically degrades to single-threaded mode.

### Graceful Degradation

The library handles thread pool initialization gracefully. If initialization fails or is unavailable:

```typescript
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

### Complete Example with Thread Pool

```typescript
import { initWasm, initThreadPool, extractBytes } from '@kreuzberg/wasm';

async function initializeKreuzbergWithThreading() {
  try {
    // Initialize WASM module
    await initWasm();

    // Initialize multi-threading
    const cpuCount = navigator.hardwareConcurrency || 1;
    try {
      await initThreadPool(cpuCount);
      console.log(`Thread pool initialized with ${cpuCount} workers`);
    } catch (error) {
      console.warn('Could not initialize thread pool, using single-threaded mode');
    }

  } catch (error) {
    console.error('Failed to initialize Kreuzberg:', error);
  }
}

async function extractDocument(file: File) {
  const bytes = new Uint8Array(await file.arrayBuffer());

  // Extraction will use multiple threads if available
  const result = await extractBytes(bytes, file.type, {
    extract_tables: true,
    extract_images: true
  });

  return result;
}

// Initialize once at app startup
await initializeKreuzbergWithThreading();

// Later, handle file uploads
fileInput.addEventListener('change', async (e) => {
  const file = e.target.files?.[0];
  if (file) {
    const result = await extractDocument(file);
    console.log('Extracted text:', result.content);
  }
});
```

### Performance Considerations

- **Thread Pool Size**: Generally, using `navigator.hardwareConcurrency` is optimal. For servers, use the number of available CPU cores.
- **Memory Usage**: Each thread has its own memory context. Large documents may require significant heap space.
- **Network Requests**: Training data and models are cached locally, so subsequent extractions are faster.

## OCR Support

The WASM binding integrates [tesseract-wasm](https://github.com/robertknight/tesseract-wasm) for OCR support with 40+ languages.

### Basic OCR

```typescript
import { extractBytes } from '@kreuzberg/wasm';

const imageBytes = await fetch('./scan.jpg').then(r => r.arrayBuffer());

const result = await extractBytes(
  new Uint8Array(imageBytes),
  'image/jpeg',
  {
    enable_ocr: true,
    ocr_config: {
      languages: ['eng'],  // English
      backend: 'tesseract-wasm'
    }
  }
);

console.log('OCR text:', result.content);
```

### Multi-Language OCR

```typescript
const result = await extractBytes(imageBytes, 'image/png', {
  enable_ocr: true,
  ocr_config: {
    languages: ['eng', 'deu', 'fra'],  // English, German, French
    backend: 'tesseract-wasm'
  }
});
```

### Supported Languages

`eng`, `deu`, `fra`, `spa`, `ita`, `por`, `nld`, `pol`, `rus`, `jpn`, `chi_sim`, `chi_tra`, `kor`, `ara`, `hin`, `tha`, `vie`, and 25+ more.

Training data is automatically loaded from jsDelivr CDN:
```
https://cdn.jsdelivr.net/npm/tesseract-wasm@0.11.0/dist/{lang}.traineddata
```

## Configuration

### Extract Tables

```typescript
import { extractBytes } from '@kreuzberg/wasm';

const result = await extractBytes(pdfBytes, 'application/pdf', {
  extract_tables: true
});

if (result.tables) {
  for (const table of result.tables) {
    console.log('Table as Markdown:');
    console.log(table.markdown);

    console.log('Table cells:');
    console.log(JSON.stringify(table.cells, null, 2));
  }
}
```

### Extract Images

```typescript
import { extractBytes } from '@kreuzberg/wasm';

const result = await extractBytes(pdfBytes, 'application/pdf', {
  extract_images: true,
  image_config: {
    target_dpi: 300,
    max_image_dimension: 4096
  }
});

if (result.images) {
  for (const image of result.images) {
    console.log(`Image ${image.index}: ${image.format}`);
    // image.data is a Uint8Array
  }
}
```

### Text Chunking

```typescript
import { extractBytes } from '@kreuzberg/wasm';

const result = await extractBytes(pdfBytes, 'application/pdf', {
  enable_chunking: true,
  chunking_config: {
    max_chars: 1000,
    max_overlap: 200
  }
});

if (result.chunks) {
  for (const chunk of result.chunks) {
    console.log(`Chunk ${chunk.index}: ${chunk.text.substring(0, 100)}...`);
  }
}
```

### Language Detection

```typescript
import { extractBytes } from '@kreuzberg/wasm';

const result = await extractBytes(pdfBytes, 'application/pdf', {
  enable_language_detection: true
});

if (result.language) {
  console.log(`Detected language: ${result.language.code}`);
  console.log(`Confidence: ${result.language.confidence}`);
}
```

### Complete Configuration Example

```typescript
import {
  extractBytes,
  type ExtractionConfig
} from '@kreuzberg/wasm';

const config: ExtractionConfig = {
  extract_tables: true,
  extract_images: true,
  extract_metadata: true,

  enable_ocr: true,
  ocr_config: {
    languages: ['eng'],
    backend: 'tesseract-wasm',
    dpi: 300,
    preprocessing: {
      deskew: true,
      denoise: true,
      binarize: true
    }
  },

  enable_chunking: true,
  chunking_config: {
    max_chars: 1000,
    max_overlap: 200
  },

  enable_language_detection: true,

  enable_quality: true,

  extract_keywords: true,
  keywords_config: {
    max_keywords: 10,
    method: 'yake'
  }
};

const result = await extractBytes(data, mimeType, config);
```

## Advanced Usage

### Batch Processing

```typescript
import { batchExtractFiles, batchExtractBytes } from '@kreuzberg/wasm';

// Browser: Process multiple files
const fileInput = document.querySelector<HTMLInputElement>('#files');
const files = Array.from(fileInput.files);

const results = await batchExtractFiles(files, {
  extract_tables: true
});

for (const result of results) {
  console.log(`${result.mime_type}: ${result.content.length} characters`);
}

// Or from Uint8Arrays
const dataList = [pdfBytes1, pdfBytes2, pdfBytes3];
const mimeTypes = ['application/pdf', 'application/pdf', 'application/pdf'];

const results = await batchExtractBytes(dataList, mimeTypes);
```

### Synchronous Extraction

```typescript
import { extractBytesSync, batchExtractBytesSync } from '@kreuzberg/wasm';

// Synchronous single extraction
const result = extractBytesSync(data, 'application/pdf', config);

// Synchronous batch extraction
const results = batchExtractBytesSync(dataList, mimeTypes, config);
```

### Plugin System

#### Custom Post-Processors

```typescript
import { registerPostProcessor } from '@kreuzberg/wasm';

registerPostProcessor({
  name: 'uppercase',
  async process(result) {
    return {
      ...result,
      content: result.content.toUpperCase()
    };
  }
});

// Now all extractions will use this processor
const result = await extractBytes(data, mimeType);
console.log(result.content); // UPPERCASE TEXT
```

#### Custom Validators

```typescript
import { registerValidator } from '@kreuzberg/wasm';

registerValidator({
  name: 'min-length',
  async validate(result) {
    if (result.content.length < 100) {
      throw new Error('Document too short');
    }
  }
});
```

#### Custom OCR Backends

```typescript
import { registerOcrBackend } from '@kreuzberg/wasm';

registerOcrBackend({
  name: 'custom-ocr',
  supportedLanguages() {
    return ['eng', 'fra'];
  },
  async initialize() {
    // Initialize your OCR backend
  },
  async processImage(imageBytes, language) {
    // Process image and return result
    return {
      content: 'extracted text',
      mime_type: 'text/plain',
      metadata: {},
      tables: []
    };
  },
  async shutdown() {
    // Cleanup
  }
});
```

### MIME Type Detection

```typescript
import {
  detectMimeFromBytes,
  getMimeFromExtension,
  getExtensionsForMime,
  normalizeMimeType
} from '@kreuzberg/wasm';

// Auto-detect MIME type from file bytes
const mimeType = detectMimeFromBytes(fileBytes);

// Get MIME type from file extension
const mime = getMimeFromExtension('pdf'); // 'application/pdf'

// Get extensions for MIME type
const extensions = getExtensionsForMime('application/pdf'); // ['pdf']

// Normalize MIME type
const normalized = normalizeMimeType('application/PDF'); // 'application/pdf'
```

### Configuration Loading

```typescript
import { loadConfigFromString } from '@kreuzberg/wasm';

// Load from YAML
const yamlConfig = `
extract_tables: true
enable_ocr: true
ocr_config:
  languages: [eng, deu]
`;
const config = loadConfigFromString(yamlConfig, 'yaml');

// Load from JSON
const jsonConfig = '{"extract_tables":true}';
const config2 = loadConfigFromString(jsonConfig, 'json');

// Load from TOML
const tomlConfig = 'extract_tables = true';
const config3 = loadConfigFromString(tomlConfig, 'toml');
```

## API Reference

### Extraction Functions

#### `extractFile(file: File, mimeType?: string, config?: ExtractionConfig): Promise<ExtractionResult>`
Extract content from a browser `File` object.

#### `extractBytes(data: Uint8Array, mimeType: string, config?: ExtractionConfig): Promise<ExtractionResult>`
Asynchronously extract content from a `Uint8Array`.

#### `extractBytesSync(data: Uint8Array, mimeType: string, config?: ExtractionConfig): ExtractionResult`
Synchronously extract content from a `Uint8Array`.

#### `batchExtractFiles(files: File[], config?: ExtractionConfig): Promise<ExtractionResult[]>`
Extract multiple files in parallel.

#### `batchExtractBytes(dataList: Uint8Array[], mimeTypes: string[], config?: ExtractionConfig): Promise<ExtractionResult[]>`
Extract multiple byte arrays in parallel.

#### `batchExtractBytesSync(dataList: Uint8Array[], mimeTypes: string[], config?: ExtractionConfig): ExtractionResult[]`
Extract multiple byte arrays synchronously.

### Plugin Management

#### Post-Processors

```typescript
registerPostProcessor(processor: PostProcessorProtocol): void
unregisterPostProcessor(name: string): void
clearPostProcessors(): void
listPostProcessors(): string[]
```

#### Validators

```typescript
registerValidator(validator: ValidatorProtocol): void
unregisterValidator(name: string): void
clearValidators(): void
listValidators(): string[]
```

#### OCR Backends

```typescript
registerOcrBackend(backend: OcrBackendProtocol): void
unregisterOcrBackend(name: string): void
clearOcrBackends(): void
listOcrBackends(): string[]
```

### Document Extractors

```typescript
listDocumentExtractors(): string[]
unregisterDocumentExtractor(name: string): void
clearDocumentExtractors(): void
```

### MIME Utilities

```typescript
detectMimeFromBytes(data: Uint8Array): string
getMimeFromExtension(ext: string): string | null
getExtensionsForMime(mime: string): string[]
normalizeMimeType(mime: string): string
```

### Configuration

```typescript
loadConfigFromString(content: string, format: 'yaml' | 'toml' | 'json'): ExtractionConfig
```

### Embeddings

```typescript
listEmbeddingPresets(): string[]
getEmbeddingPreset(name: string): EmbeddingPreset | null
```

## Types

All types are shared via the `@kreuzberg/core` package:

```typescript
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
  PostProcessorProtocol,
  ValidatorProtocol,
  OcrBackendProtocol
} from '@kreuzberg/core';
```

### ExtractionResult

Main result object containing:
- `content: string` - Extracted text content
- `mime_type: string` - MIME type of the document
- `metadata?: Metadata` - Document metadata
- `tables?: Table[]` - Extracted tables
- `images?: ExtractedImage[]` - Extracted images
- `chunks?: Chunk[]` - Text chunks (if chunking enabled)
- `language?: LanguageInfo` - Detected language (if enabled)
- `keywords?: Keyword[]` - Extracted keywords (if enabled)

### ExtractionConfig

Configuration object for extraction:
- `extract_tables?: boolean` - Extract tables as structured data
- `extract_images?: boolean` - Extract embedded images
- `extract_metadata?: boolean` - Extract document metadata
- `enable_ocr?: boolean` - Enable OCR for images
- `ocr_config?: OcrConfig` - OCR settings
- `enable_chunking?: boolean` - Split text into semantic chunks
- `chunking_config?: ChunkingConfig` - Text chunking settings
- `enable_language_detection?: boolean` - Detect document language
- `enable_quality?: boolean` - Encoding detection, normalization
- `extract_keywords?: boolean` - Extract important keywords
- `keywords_config?: KeywordsConfig` - Keyword extraction settings

### Table

Extracted table structure:
- `markdown: string` - Table in Markdown format
- `cells: TableCell[][]` - 2D array of table cells
- `row_count: number` - Number of rows
- `column_count: number` - Number of columns

## Supported Formats

| Category | Formats |
|----------|---------|
| **Documents** | PDF, DOCX, DOC, PPTX, PPT, XLSX, XLS, ODT, ODP, ODS, RTF |
| **Images** | PNG, JPEG, JPG, WEBP, BMP, TIFF, GIF |
| **Web** | HTML, XHTML, XML, EPUB |
| **Text** | TXT, MD, RST, LaTeX, CSV, TSV, JSON, YAML, TOML, ORG, BIB, TYP, FB2 |
| **Email** | EML, MSG |
| **Archives** | ZIP, TAR, 7Z |
| **Other** | And 30+ more formats |

## Build from Source

### Prerequisites

- Rust 1.75+ with `wasm32-unknown-unknown` target
- Node.js 18+ with pnpm
- wasm-pack

```bash
# Install Rust target
rustup target add wasm32-unknown-unknown

# Install wasm-pack
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# Build WASM package
cd crates/kreuzberg-wasm
pnpm install
pnpm run build

# Run tests
pnpm test
```

### Build Targets

```bash
# For browsers (ESM modules)
pnpm run build:wasm:web

# For bundlers (webpack, rollup, vite)
pnpm run build:wasm:bundler

# For Node.js
pnpm run build:wasm:nodejs

# For Deno
pnpm run build:wasm:deno

# Build all targets
pnpm run build:all
```

## Limitations

### No File System Access

The WASM binding cannot access the file system directly. Use file readers:

```typescript
// ❌ Won't work
await extractFileSync('./document.pdf');  // Throws error

// ✅ Use file readers instead
const bytes = await Deno.readFile('./document.pdf');  // Deno
const bytes = await fs.readFile('./document.pdf');    // Node.js
const bytes = await file.arrayBuffer();                // Browser
```

### OCR Training Data

Tesseract training data (`.traineddata` files) are loaded from jsDelivr CDN on first use. For offline usage or custom CDN, see the [OCR documentation](https://kreuzberg.dev).

### Size Constraints

Cloudflare Workers has a 10MB bundle size limit (compressed). The WASM binary is ~2MB compressed, leaving room for your application code.

## Troubleshooting

### "WASM module failed to initialize"

Ensure your bundler is configured to handle WASM files:

**Vite:**
```typescript
// vite.config.ts
export default {
  optimizeDeps: {
    exclude: ['@kreuzberg/wasm']
  }
}
```

**Webpack:**
```javascript
// webpack.config.js
module.exports = {
  experiments: {
    asyncWebAssembly: true
  }
}
```

### "Module not found: @kreuzberg/core"

The @kreuzberg/core package is a peer dependency. Install it:

```bash
pnpm add @kreuzberg/core
```

### Memory Issues in Workers

For large documents in Cloudflare Workers, process in smaller chunks:

```typescript
const result = await extractBytes(pdfBytes, 'application/pdf', {
  chunking_config: { max_chars: 1000 }
});
```

### OCR Not Working

Check that tesseract-wasm is loading correctly. The training data is automatically fetched from CDN on first use.

## Examples

See the [`examples/`](./examples/) directory for complete working examples:

- **Browser**: Vanilla JS file upload interface
- **Deno**: Command-line document extraction
- **Cloudflare Workers**: Document processing API
- **Node.js**: Batch processing script

## Documentation

For comprehensive documentation, visit [https://kreuzberg.dev](https://kreuzberg.dev)

## Contributing

We welcome contributions! Please see our [Contributing Guide](https://github.com/kreuzberg-dev/kreuzberg/blob/main/docs/contributing.md) for details.

## License

MIT

## Links

- [Website](https://kreuzberg.dev)
- [Documentation](https://kreuzberg.dev)
- [GitHub](https://github.com/kreuzberg-dev/kreuzberg)
- [Issue Tracker](https://github.com/kreuzberg-dev/kreuzberg/issues)
- [Changelog](https://github.com/kreuzberg-dev/kreuzberg/blob/main/CHANGELOG.md)
- [npm Package](https://www.npmjs.com/package/@kreuzberg/wasm)

## Related Packages

- [@kreuzberg/node](https://www.npmjs.com/package/@kreuzberg/node) - Native Node.js bindings (NAPI)
- [@kreuzberg/core](https://www.npmjs.com/package/@kreuzberg/core) - Shared TypeScript types
- [kreuzberg](https://crates.io/crates/kreuzberg) - Rust core library
