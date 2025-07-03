# API Server

Kreuzberg includes a built-in REST API server powered by [Litestar](https://litestar.dev/) for document extraction over HTTP.

## Installation

Install Kreuzberg with the API extra:

```bash
pip install "kreuzberg[api]"
```

## Running the API Server

### Using Python

```python
from kreuzberg._api.main import app
import uvicorn

uvicorn.run(app, host="0.0.0.0", port=8000)
```

### Using Litestar CLI

```bash
litestar --app kreuzberg._api.main:app run
```

### With Custom Settings

```bash
litestar --app kreuzberg._api.main:app run --host 0.0.0.0 --port 8080
```

## API Endpoints

### Health Check

```bash
GET /health
```

Returns the server status:

```json
{
  "status": "ok"
}
```

### Extract Files

```bash
POST /extract
```

Extract text from one or more files.

**Request:**

- Method: `POST`
- Content-Type: `multipart/form-data`
- Body: One or more files with field name `data`

**Response:**

- Status: 201 Created
- Body: Array of extraction results

**Example:**

```bash
# Single file
curl -X POST http://localhost:8000/extract \
  -F "data=@document.pdf"

# Multiple files
curl -X POST http://localhost:8000/extract \
  -F "data=@document1.pdf" \
  -F "data=@document2.docx" \
  -F "data=@image.jpg"
```

**Response Format:**

```json
[
  {
    "content": "Extracted text content...",
    "mime_type": "text/plain",
    "metadata": {
      "pages": 5,
      "title": "Document Title"
    }
  }
]
```

## Error Handling

The API uses standard HTTP status codes:

- `200 OK`: Successful health check
- `201 Created`: Successful extraction
- `400 Bad Request`: Validation error (e.g., invalid file format)
- `422 Unprocessable Entity`: Parsing error (e.g., corrupted file)
- `500 Internal Server Error`: Unexpected error

Error responses include:

```json
{
  "message": "Error description",
  "details": "{\"additional\": \"context\"}"
}
```

## Features

- **Batch Processing**: Extract from multiple files in a single request
- **Automatic Format Detection**: Detects file types from MIME types
- **OCR Support**: Automatically applies OCR to images and scanned PDFs
- **Structured Logging**: Uses structlog for detailed logging
- **OpenTelemetry**: Built-in observability support
- **Async Processing**: High-performance async request handling

## Configuration

The API server uses the default Kreuzberg extraction configuration:

- Tesseract OCR is included by default
- PDF, image, and document extraction is supported
- Table extraction with GMFT (if installed)

To use custom configuration, modify the extraction call in your own API wrapper:

```python
from kreuzberg import ExtractionConfig, batch_extract_bytes
from litestar import Litestar, post

@post("/extract-custom")
async def custom_extract(data: list[UploadFile]) -> list[ExtractionResult]:
    config = ExtractionConfig(force_ocr=True, ocr_backend="easyocr", extract_tables=True)
    return await batch_extract_bytes([(await file.read(), file.content_type) for file in data], config=config)

app = Litestar(route_handlers=[custom_extract])
```

## Production Deployment

For production use, consider:

1. **Reverse Proxy**: Use nginx or similar for SSL termination
1. **Process Manager**: Use systemd, supervisor, or similar
1. **Workers**: Run multiple workers with uvicorn or gunicorn
1. **Monitoring**: Enable OpenTelemetry exporters
1. **Rate Limiting**: Add rate limiting middleware
1. **Authentication**: Add authentication middleware if needed

Example production command:

```bash
uvicorn kreuzberg._api.main:app \
  --host 0.0.0.0 \
  --port 8000 \
  --workers 4 \
  --log-level info
```
