# Kreuzberg WASM Examples

Complete working examples demonstrating document intelligence capabilities across different JavaScript/TypeScript runtimes.

## Overview

Kreuzberg WASM provides a consistent API for document extraction in browsers, servers, and serverless environments. This directory contains production-ready examples for common deployment patterns.

## Quick Comparison

| Feature | Deno | Cloudflare Workers | Browser |
|---------|------|-------------------|---------|
| **Environment** | Server (Deno runtime) | Serverless (Edge) | Client-side (Vite) |
| **Use Case** | CLI tools, scripts, server apps | API endpoints, edge computing | Web applications, UI |
| **File Access** | Local filesystem | HTTP form upload | User file selection |
| **Deployment** | VPS, Docker | Cloudflare platform | Static hosting (Vercel, Netlify) |
| **Multi-threading** | Yes (wasm-bindgen-rayon) | Yes (with headers) | Yes (with COOP/COEP) |
| **Cold Start** | N/A | Sub-millisecond | N/A |
| **Memory Limits** | Unlimited | ~128MB per request | Browser heap |
| **Latency** | Varies | <50ms typical | Instant (local) |

## Example Directories

### Deno Example
**Location:** [`../wasm-deno/`](../wasm-deno)

Server-side document extraction using the Deno runtime. Demonstrates core API usage with synchronous and asynchronous extraction.

**Key Features:**
- Basic text extraction from PDFs and documents
- Batch processing multiple files efficiently
- OCR support for scanned documents
- Command-line file processing
- Error handling and edge cases

**Running:**
```bash
cd wasm-deno
deno run --allow-read basic.ts
deno run --allow-read batch.ts
deno run --allow-read ocr.ts
```

**Best For:**
- Backend document processing services
- CLI tools and automation scripts
- Batch file conversion pipelines
- Testing and development

---

### Cloudflare Workers Example
**Location:** [`../wasm-cloudflare-workers/`](../wasm-cloudflare-workers)

Production-ready serverless API for document processing on Cloudflare's edge network. Includes HTTP endpoints, file upload handling, and error management.

**Key Features:**
- HTTP POST endpoint for document uploads
- Multipart form data handling
- Streaming JSON responses
- CORS support for browser requests
- Health checks and API documentation
- Production-ready error handling
- Deployable to Cloudflare Workers

**Running:**
```bash
cd wasm-cloudflare-workers
npm install
npm run dev  # Local development server
npm run deploy  # Deploy to Cloudflare
```

**API Endpoints:**
- `GET /health` - Health check
- `GET /` - API documentation
- `POST /extract` - Document extraction with file upload

**Best For:**
- REST APIs for document processing
- Microservices on the edge
- Webhook integrations
- Serverless architectures

---

### Browser Example
**Location:** [`../wasm-browser/`](../wasm-browser)

Interactive web application with modern UI for document extraction. Built with Vite, includes drag-and-drop upload, real-time progress, and multi-threaded processing.

**Key Features:**
- Drag-and-drop file upload interface
- Real-time progress indication
- Support for multiple document formats
- Extracted text with syntax highlighting
- JSON metadata viewer
- Copy to clipboard and download functionality
- Dark mode support
- Mobile-responsive design
- Multi-threading with proper COOP/COEP headers

**Running:**
```bash
cd wasm-browser
pnpm install
pnpm dev  # Development server (http://localhost:5173)
pnpm build  # Production build
```

**Best For:**
- SaaS applications
- Document management tools
- Internal enterprise tools
- Web-based document processors

---

## Getting Started

### Installation

Each example is independent with its own dependencies. Install and run your chosen example:

```bash
# Deno - no installation needed (installed globally)
cd examples/wasm-deno
deno run --allow-read basic.ts

# Cloudflare Workers
cd examples/wasm-cloudflare-workers
npm install
npm run dev

# Browser
cd examples/wasm-browser
pnpm install
pnpm dev
```

### Requirements

**Deno:**
- Deno 1.0+ ([install](https://deno.land))

**Cloudflare Workers:**
- Node.js 18+
- pnpm or npm
- Cloudflare account (for deployment)

**Browser:**
- Node.js 18+
- pnpm (required by this project)
- Modern web browser (Chrome 93+, Firefox 79+, Safari 16.4+, Edge 93+)

## Common Setup

### 1. Prepare Test Documents

Each example includes sample documents in `fixtures/` or `public/` directories:

```bash
# Copy test documents to example directory
cp /path/to/test.pdf examples/wasm-deno/fixtures/
cp /path/to/test.pdf examples/wasm-browser/public/
```

### 2. API Usage Pattern

All examples follow the same core API pattern:

```typescript
import { extractBytes } from '@kreuzberg/wasm';

// Read your document
const fileBytes = await readFile('./document.pdf');

// Extract content
const result = await extractBytes(fileBytes, 'application/pdf', {
  extract_tables: true,
  extract_images: true
});

// Use results
console.log('Text:', result.content);
console.log('Metadata:', result.metadata);
console.log('Tables:', result.tables);
```

### 3. Configuration

All examples support the same extraction configuration:

```typescript
import { ExtractionConfig } from '@kreuzberg/core';

const config: ExtractionConfig = {
  extract_tables: true,
  extract_images: true,
  extract_metadata: true,
  enable_ocr: true,
  ocr_config: {
    languages: ['eng', 'deu'],
    backend: 'tesseract-wasm'
  }
};
```

### 4. Error Handling

Consistent error handling pattern across all examples:

```typescript
try {
  const result = await extractBytes(data, mimeType, config);
  // Process result
} catch (error) {
  if (error instanceof Error) {
    console.error(`Extraction failed: ${error.message}`);
  }
}
```

## Troubleshooting

### Module Not Found

**Problem:** "Cannot find module @kreuzberg/wasm"

**Solution:**
```bash
# Install in your example directory
npm install @kreuzberg/wasm
# or for pnpm
pnpm add @kreuzberg/wasm
```

### WASM Module Failed to Initialize

**Problem:** "WASM module failed to initialize" in browser example

**Solution:**
- Ensure COOP/COEP headers are set (automatic in Vite dev server)
- Verify WASM is bundled correctly
- Check browser console for specific errors

### Memory Issues

**Problem:** "Out of memory" errors with large documents

**Solutions:**
- Use chunking for large documents
- Process in smaller batches
- Increase available system memory
- For Cloudflare Workers, upgrade plan for higher memory limits

### OCR Not Working

**Problem:** OCR extraction returns no text

**Solutions:**
- Ensure tesseract-wasm is loaded (CDN fetch)
- Check browser console for fetch errors
- Verify language code (e.g., 'eng', 'deu', 'fra')
- For offline use, pre-download training data

### Deno Permission Errors

**Problem:** "Permission denied" when running scripts

**Solution:**
```bash
# Grant required permissions
deno run --allow-read --allow-write basic.ts
deno run --allow-net basic.ts  # For network access
```

### Cloudflare Worker Timeout

**Problem:** "Processing timeout" on large files

**Solutions:**
1. Use smaller files for testing
2. Upgrade to Pro/Business plan (10s → 30s limit)
3. Implement chunked processing
4. Pre-process files on client before upload

## Performance Tips

### Optimize for Your Environment

**Deno:**
- Use `extractBytesSync()` for simple operations
- Use batch processing for multiple files
- Leverage Deno's built-in caching

**Cloudflare Workers:**
- Keep file sizes under 50MB
- Stream large responses
- Cache results when possible
- Monitor CPU time usage

**Browser:**
- Initialize thread pool early
- Use web workers for non-blocking UI
- Implement progress feedback
- Cache WASM module in service worker

### Multi-threading Configuration

```typescript
import { initThreadPool } from '@kreuzberg/wasm';

// Initialize once at startup
const cpuCount = navigator.hardwareConcurrency || 4;
await initThreadPool(cpuCount);

// Subsequent extractions use multiple threads
const result = await extractBytes(data, mimeType);
```

## Supported Formats

All examples support the same document formats:

| Category | Formats |
|----------|---------|
| **Documents** | PDF, DOCX, DOC, PPTX, PPT, XLSX, XLS, ODT, ODP, ODS, RTF |
| **Images** | PNG, JPEG, JPG, WEBP, BMP, TIFF, GIF |
| **Web** | HTML, XHTML, XML, EPUB |
| **Text** | TXT, MD, RST, LaTeX, CSV, TSV, JSON, YAML, TOML, ORG |
| **Email** | EML, MSG |
| **Archives** | ZIP, TAR, 7Z |

## Advanced Usage

### Custom Post-Processors

Register custom transformations for all examples:

```typescript
import { registerPostProcessor } from '@kreuzberg/wasm';

registerPostProcessor({
  name: 'my-processor',
  async process(result) {
    return {
      ...result,
      content: result.content.toLowerCase()
    };
  }
});
```

### Batch Processing

All examples support batch operations:

```typescript
import { batchExtractBytes } from '@kreuzberg/wasm';

const results = await batchExtractBytes(
  [file1Bytes, file2Bytes, file3Bytes],
  ['application/pdf', 'application/pdf', 'text/plain']
);
```

### Configuration Loading

Load extraction configuration from files:

```typescript
import { loadConfigFromString } from '@kreuzberg/wasm';

const yamlConfig = `
extract_tables: true
enable_ocr: true
ocr_config:
  languages: [eng, deu]
`;

const config = loadConfigFromString(yamlConfig, 'yaml');
const result = await extractBytes(data, mimeType, config);
```

## Deployment

### Deno Example

Deploy with Docker or systemd:

```dockerfile
FROM denoland/deno:latest
WORKDIR /app
COPY . .
RUN deno cache --allow-import basic.ts
CMD ["deno", "run", "--allow-read", "basic.ts"]
```

### Cloudflare Workers Example

```bash
cd examples/wasm-cloudflare-workers
npm run deploy
```

### Browser Example

**Vercel:**
```bash
cd examples/wasm-browser
vercel deploy
```

**Netlify:**
```bash
cd examples/wasm-browser
netlify deploy
```

**Self-hosted (Nginx):**
```nginx
server {
    listen 80;
    root /var/www/html/dist;

    # WASM headers for multi-threading
    add_header Cross-Origin-Opener-Policy "same-origin";
    add_header Cross-Origin-Embedder-Policy "require-corp";

    location / {
        try_files $uri /index.html;
    }
}
```

## Testing

Each example includes sample documents and can be tested locally:

```bash
# Deno - test with included fixtures
cd examples/wasm-deno
deno run --allow-read basic.ts

# Cloudflare Workers - test API locally
cd examples/wasm-cloudflare-workers
npm run dev
curl -X POST -F "file=@test.pdf" http://localhost:8787/extract

# Browser - test UI locally
cd examples/wasm-browser
pnpm dev
# Open http://localhost:5173 and upload a file
```

## API Reference

See the main [Kreuzberg WASM README](../../crates/kreuzberg-wasm/README.md) for comprehensive API documentation.

### Key Functions

- `extractBytes(data, mimeType, config?)` - Async extraction
- `extractBytesSync(data, mimeType, config?)` - Sync extraction
- `batchExtractBytes(dataList, mimeTypes, config?)` - Batch async
- `initThreadPool(numWorkers)` - Initialize multi-threading

### Configuration

All examples use the same `ExtractionConfig` type with options for:
- Table extraction (`extract_tables`)
- Image extraction (`extract_images`)
- OCR (`enable_ocr`, `ocr_config`)
- Text chunking (`enable_chunking`, `chunking_config`)
- Metadata extraction (`extract_metadata`)
- Language detection (`enable_language_detection`)

## Resources

- **[Kreuzberg Documentation](https://kreuzberg.dev)**
- **[npm Package: @kreuzberg/wasm](https://www.npmjs.com/package/@kreuzberg/wasm)**
- **[GitHub Repository](https://github.com/kreuzberg-dev/kreuzberg)**

### Runtime Documentation

- **[Deno Manual](https://docs.deno.com)**
- **[Cloudflare Workers](https://developers.cloudflare.com/workers/)**
- **[Vite Guide](https://vitejs.dev)**

### WebAssembly

- **[WebAssembly MDN Docs](https://developer.mozilla.org/en-US/docs/WebAssembly/)**
- **[wasm-bindgen-rayon](https://docs.rs/wasm-bindgen-rayon/)**
- **[COOP/COEP Guide](https://developer.chrome.com/docs/security/cross-origin-policy/)**

## License

All examples are part of the Kreuzberg project. See the main repository LICENSE file.

## Contributing

We welcome contributions! Please see the main repository's contributing guide for details.

## Questions?

- Open an issue on [GitHub](https://github.com/kreuzberg-dev/kreuzberg/issues)
- Check the [discussion forum](https://github.com/kreuzberg-dev/kreuzberg/discussions)
- Read the full [documentation](https://kreuzberg.dev)
