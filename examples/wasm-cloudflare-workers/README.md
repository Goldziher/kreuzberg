# Kreuzberg WASM Cloudflare Workers Example

A production-ready example demonstrating how to deploy Kreuzberg document intelligence as a serverless API on Cloudflare Workers.

## Features

- **HTTP File Upload API**: Accept document uploads via `POST /extract`
- **Binary Data Handling**: Efficient processing of ArrayBuffer and multipart/form-data
- **Streaming JSON Responses**: Fast response streaming with proper JSON encoding
- **Error Handling**: Comprehensive error handling with proper HTTP status codes
- **CORS Support**: Browser-friendly headers for cross-origin requests
- **Health Checks**: Simple health check endpoint at `GET /health`
- **API Documentation**: Auto-generated API docs at `GET /`
- **Production Ready**: Fully typed with TypeScript, tested configuration

## Supported Document Formats

- **Documents**: PDF, DOCX, XLSX, PPTX, HTML
- **Images**: JPG, PNG, GIF, WebP (with OCR)
- **Text**: TXT, HTML

## Getting Started

### Prerequisites

- Node.js 18+ (LTS recommended)
- pnpm or npm
- Cloudflare account (for deployment)

### Installation

```bash
cd examples/wasm-cloudflare-workers
npm install
# or
pnpm install
```

### Local Development

Start the development server:

```bash
npm run dev
```

The worker will be available at `http://localhost:8787`

Test the API:

```bash
# Health check
curl http://localhost:8787/health

# API documentation
curl http://localhost:8787/

# Extract from a document
curl -X POST -F "file=@sample.pdf" http://localhost:8787/extract
```

### Type Checking

Verify TypeScript compilation:

```bash
npm run typecheck
```

## API Endpoints

### `GET /health`

Health check endpoint.

**Response:**
```json
{
  "status": "healthy",
  "timestamp": "2024-11-15T10:30:00.000Z",
  "version": "1.0.0"
}
```

### `GET /`

API documentation and available endpoints.

**Response:**
```json
{
  "name": "Kreuzberg WASM API",
  "version": "1.0.0",
  "endpoints": { ... },
  "supportedFormats": ["pdf", "docx", "xlsx", "pptx", "html", "image"]
}
```

### `POST /extract`

Extract text and structured data from documents.

**Request:**
- Content-Type: `multipart/form-data`
- Form field: `file` (required) - Document file to process

**Example:**
```bash
curl -X POST \
  -F "file=@document.pdf" \
  http://localhost:8787/extract \
  | jq .
```

**Response:**
```json
{
  "text": "Extracted text content...",
  "metadata": {
    "title": "Document Title",
    "author": "Author Name",
    "pages": 10
  },
  "tables": [
    {
      "headers": ["Column 1", "Column 2"],
      "rows": [
        ["Cell 1", "Cell 2"],
        ["Cell 3", "Cell 4"]
      ]
    }
  ]
}
```

**Status Codes:**
- `200 OK` - Extraction successful
- `400 Bad Request` - Missing or invalid file
- `404 Not Found` - Unknown endpoint
- `413 Payload Too Large` - File exceeds 100MB
- `415 Unsupported Media Type` - File format not supported
- `500 Internal Server Error` - Processing error
- `504 Gateway Timeout` - Processing took too long
- `507 Insufficient Storage` - Insufficient memory

## Error Responses

All errors return JSON with consistent format:

```json
{
  "error": "Human readable error message",
  "code": "ERROR_CODE",
  "details": "Optional detailed information"
}
```

**Common Error Codes:**
- `MISSING_FILE` - No file provided
- `FILE_TOO_LARGE` - File exceeds size limit
- `UNSUPPORTED_FILE_TYPE` - Document format not supported
- `PROCESSING_ERROR` - Error during extraction
- `PROCESSING_TIMEOUT` - Processing exceeded time limit
- `INSUFFICIENT_MEMORY` - Insufficient memory
- `NOT_FOUND` - Endpoint not found

## Deployment

### Deploy to Cloudflare

```bash
npm run deploy
```

This will deploy to your Cloudflare account. You'll need:
1. Cloudflare account credentials
2. Domain configured in Cloudflare (for production)

### Configure Production

Edit `wrangler.toml` to set your production domain:

```toml
[env.production]
route = "https://api.yourdomain.com/*"
zone_id = "your-zone-id"
```

Then deploy to production:

```bash
npm run deploy -- --env production
```

## Configuration

### File Size Limits

Default: 100MB. Modify in `src/index.ts`:

```typescript
const MAX_FILE_SIZE = 100 * 1024 * 1024; // Change this value
```

### CORS Configuration

Modify `CORS_HEADERS` in `src/index.ts` to restrict origins:

```typescript
const CORS_HEADERS = {
  'Access-Control-Allow-Origin': 'https://yourdomain.com',
  // ... other headers
};
```

### Timeouts

Cloudflare Workers have different timeout limits:
- **Free tier**: 10 seconds CPU time
- **Pro/Business**: 30 seconds CPU time

For large files, consider:
1. Using smaller files for testing
2. Upgrading to Pro/Business plan
3. Implementing chunked processing

## Performance Considerations

### Cold Starts

Cloudflare Workers provide sub-millisecond startup times. WASM module loading happens once per worker isolation.

### Memory Usage

The WASM module and processing consume:
- ~30MB base module size
- ~50-100MB per large document extraction
- Available memory: Worker isolated context (~128MB)

### Optimization Tips

1. **Stream large responses** - Already handled by default
2. **Compress files** - Send pre-compressed files when possible
3. **Monitor logs** - Use Cloudflare dashboard for metrics
4. **Cache results** - Implement result caching for repeated documents

## Testing

### Local Testing

Create a test document and run:

```bash
npm run dev
# In another terminal
curl -X POST -F "file=@test-document.pdf" http://localhost:8787/extract
```

### Browser Testing

The API supports CORS. Test from browser console:

```javascript
const formData = new FormData();
const fileInput = document.querySelector('input[type="file"]');
formData.append('file', fileInput.files[0]);

const response = await fetch('http://localhost:8787/extract', {
  method: 'POST',
  body: formData,
});

const data = await response.json();
console.log(data);
```

## Development

### Project Structure

```
src/
├── index.ts          # Main worker handler with all endpoints
└── types/            # TypeScript type definitions
```

### Code Style

Code is formatted with Prettier:

```bash
npm run format
```

### Type Safety

Full TypeScript support with strict mode enabled.

## Troubleshooting

### "Module not found: @kreuzberg/wasm"

Ensure the package is installed:
```bash
npm install @kreuzberg/wasm
```

### "WASM module not available"

The WASM module requires proper bundler support. Make sure:
1. You're using the correct Node.js version (18+)
2. The bundler is configured properly (handled by wrangler)
3. WASM modules are available in node_modules

### Processing Timeouts

If files consistently timeout:
1. Try with smaller files first
2. Check available CPU time (upgrade plan if needed)
3. Monitor worker logs in Cloudflare dashboard

### Large File Issues

Cloudflare Workers have memory limits:
- Free/Pro: ~128MB available per request
- Business: Higher limits available

For files >50MB, consider:
1. Preprocessing/compression on client
2. Using the Node.js version of Kreuzberg for larger batches
3. Implementing chunked processing

## Security Considerations

### Input Validation

The example includes:
- File size validation (100MB limit)
- File type validation
- Form data validation

### CORS

Default configuration allows all origins. For production:

```typescript
const CORS_HEADERS = {
  'Access-Control-Allow-Origin': 'https://trusted-domain.com',
  // ... restrict as needed
};
```

### Rate Limiting

Implement using Cloudflare's rate limiting rules in dashboard:
1. Go to Workers Routes
2. Configure rate limiting rules
3. Set thresholds per IP/path

## License

MIT - See LICENSE file in root repository

## Additional Resources

- [Cloudflare Workers Documentation](https://developers.cloudflare.com/workers/)
- [Wrangler CLI Documentation](https://developers.cloudflare.com/workers/wrangler/install-and-update/)
- [Kreuzberg Documentation](https://kreuzberg.dev)
- [WebAssembly in JavaScript](https://developer.mozilla.org/en-US/docs/WebAssembly)
