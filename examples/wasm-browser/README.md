# Kreuzberg WASM Browser Example

A complete, production-ready browser application for document extraction using the Kreuzberg WASM library with Vite.

## Features

- **File Upload**: Drag-and-drop or click to upload documents
- **Real-time Progress**: Visual progress indication during extraction
- **Multiple Formats**: PDF, DOCX, XLSX, HTML, PNG, JPEG support
- **Content Display**: Syntax-highlighted extracted text
- **Metadata Viewing**: JSON-formatted metadata inspection
- **Copy & Download**: Copy text to clipboard or download as `.txt`
- **Error Handling**: User-friendly error messages
- **Dark Mode**: Automatic dark mode support
- **Mobile Responsive**: Works on all device sizes
- **Multi-threading**: Uses `wasm-bindgen-rayon` for parallel processing

## Important: COOP/COEP Headers

This application requires special HTTP headers to enable `SharedArrayBuffer` support for WebAssembly multi-threading:

```
Cross-Origin-Opener-Policy: same-origin
Cross-Origin-Embedder-Policy: require-corp
```

These headers are **automatically set by the Vite development server**. For production deployment, configure these headers in your web server or CDN:

### Nginx
```nginx
add_header Cross-Origin-Opener-Policy "same-origin";
add_header Cross-Origin-Embedder-Policy "require-corp";
```

### Cloudflare
Use `_headers` file or Page Rules with custom headers.

### AWS CloudFront
Add custom headers via Origin Response Lambda function.

### Vercel
Configure in `vercel.json`:
```json
{
  "headers": [
    {
      "source": "/(.*)",
      "headers": [
        {
          "key": "Cross-Origin-Opener-Policy",
          "value": "same-origin"
        },
        {
          "key": "Cross-Origin-Embedder-Policy",
          "value": "require-corp"
        }
      ]
    }
  ]
}
```

## Setup

### Prerequisites

- Node.js 18+
- pnpm (required by this project)

### Installation

```bash
pnpm install
```

## Development

Start the Vite development server:

```bash
pnpm dev
```

The application will open automatically at `http://localhost:5173`. COOP/COEP headers are automatically configured.

## Building

Build for production:

```bash
pnpm build
```

This generates an optimized bundle in the `dist` directory.

## Project Structure

```
wasm-browser/
├── index.html              # Entry HTML with COOP/COEP meta tags
├── vite.config.ts          # Vite configuration with COOP/COEP headers
├── tsconfig.json           # TypeScript configuration
├── package.json            # Dependencies
├── README.md              # This file
├── .gitignore             # Git ignore rules
├── public/
│   └── sample.pdf         # Sample document for testing
└── src/
    ├── main.ts            # Application entry point
    ├── styles.css         # Comprehensive styling
    └── types.ts           # Type definitions
```

## How It Works

1. **Upload Document**: Drag-and-drop a file or click to browse, or click "Load Sample PDF"
2. **Processing**: The WASM library extracts content and metadata in parallel
3. **Progress**: Real-time progress bar shows extraction status
4. **Results**: View extracted text and metadata in separate tabs
5. **Export**: Copy to clipboard or download as plain text

## API Usage

The application uses the `@kreuzberg/wasm` package:

```typescript
import { extractBytes } from "@kreuzberg/wasm";

const result = await extractBytes(fileData, mimeType);
// result.content - extracted text
// result.mimeType - detected MIME type
// result.metadata - document metadata
```

## Supported Document Types

| Format | MIME Type |
|--------|-----------|
| PDF | `application/pdf` |
| Word (.docx) | `application/vnd.openxmlformats-officedocument.wordprocessingml.document` |
| Excel (.xlsx) | `application/vnd.openxmlformats-officedocument.spreadsheetml.sheet` |
| HTML | `text/html` |
| PNG | `image/png` |
| JPEG | `image/jpeg` |
| Plain Text | `text/plain` |

## Performance Tips

- **Large Files**: For very large documents (>100MB), consider implementing file chunking
- **Memory**: The WASM module requires sufficient heap memory; most browsers allocate this automatically
- **Threading**: Multi-threaded processing is automatic via `wasm-bindgen-rayon`

## Troubleshooting

### SharedArrayBuffer Error

If you see `SharedArrayBuffer is not defined`:
1. Ensure COOP/COEP headers are properly configured
2. Check browser console for CSP errors
3. Verify the application is served over HTTPS (if required by your deployment)

### File Upload Fails

1. Check file size limits (browser typically allows up to 2GB)
2. Verify MIME type is in the supported list
3. Check browser console for detailed error messages

### Slow Performance

1. Reduce file size if possible
2. Close other applications to free memory
3. Check browser DevTools for performance bottlenecks

## Browser Support

- Chrome 93+ (with COOP/COEP headers)
- Firefox 79+ (with COOP/COEP headers)
- Safari 16.4+ (with COOP/COEP headers)
- Edge 93+ (with COOP/COEP headers)

## Development Notes

- The `vite.config.ts` defines both build and dev server configurations with COOP/COEP headers
- The `index.html` includes meta tags for COOP/COEP in addition to server headers (defense in depth)
- All TypeScript is strictly typed with no implicit `any` types
- Styles use CSS Grid and Flexbox for responsive, accessible design
- No external UI frameworks—all vanilla TypeScript and CSS
- Multi-threading is handled automatically by `wasm-bindgen-rayon`
- Dark mode support via `prefers-color-scheme` media query

## License

This example is part of the Kreuzberg project. See the main repository for license information.

## Resources

- [Kreuzberg Documentation](https://github.com/julien-cros/kreuzberg)
- [Vite Documentation](https://vitejs.dev)
- [WebAssembly MDN Docs](https://developer.mozilla.org/en-US/docs/WebAssembly)
- [SharedArrayBuffer Security](https://developer.chrome.com/docs/security/cross-origin-policy/)
