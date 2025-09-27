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
- **Maximum file size: Configurable via `KREUZBERG_MAX_UPLOAD_SIZE` environment variable (default: 1GB per file)**

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
    },
    "chunks": [],
    "entities": null,
    "keywords": null,
    "detected_languages": null
  }
]
```

### Runtime Configuration

The `/extract` endpoint supports runtime configuration via query parameters and HTTP headers, allowing you to customize extraction behavior without requiring static configuration files.

#### Query Parameters

Configure extraction options directly via URL query parameters:

Enable chunking with custom settings:

```bash
curl -X POST "http://localhost:8000/extract?chunk_content=true&max_chars=500&max_overlap=50" \
  -F "data=@document.pdf"
```

Extract entities and keywords:

```bash
curl -X POST "http://localhost:8000/extract?extract_entities=true&extract_keywords=true&keyword_count=5" \
  -F "data=@document.pdf"
```

Force OCR with specific backend:

```bash
curl -X POST "http://localhost:8000/extract?force_ocr=true&ocr_backend=tesseract" \
  -F "data=@image.jpg"
```

### Image Extraction and OCR

Kreuzberg can extract embedded images from various document formats and optionally run OCR on them to extract text content:

```bash
# Basic image extraction from PDF documents
curl -X POST "http://localhost:8000/extract?extract_images=true" \
  -F "data=@document.pdf"

# Extract images and run OCR on them using Tesseract
curl -X POST "http://localhost:8000/extract?extract_images=true&ocr_extracted_images=true&image_ocr_backend=tesseract" \
  -F "data=@scanned_document.pdf"

# Extract images from PowerPoint presentations with dimension filtering
curl -X POST "http://localhost:8000/extract?extract_images=true&ocr_extracted_images=true&image_ocr_min_width=100&image_ocr_min_height=100&image_ocr_max_width=3000&image_ocr_max_height=3000" \
  -F "data=@presentation.pptx"

# Use EasyOCR for better scene text recognition
curl -X POST "http://localhost:8000/extract?extract_images=true&ocr_extracted_images=true&image_ocr_backend=easyocr" \
  -F "data=@document_with_photos.pdf"

# Extract images from HTML with inline base64 images
curl -X POST "http://localhost:8000/extract?extract_images=true" \
  -F "data=@webpage.html"

# Process multiple documents with different image extraction settings
curl -X POST "http://localhost:8000/extract?extract_images=true&ocr_extracted_images=true&image_ocr_backend=tesseract" \
  -F "data=@document1.pdf" \
  -F "data=@presentation.pptx" \
  -F "data=@email.eml"
```

**Image Extraction Response Format:**

When image extraction is enabled, the response includes additional fields:

```json
[
  {
    "content": "Main document text content...",
    "mime_type": "text/plain",
    "metadata": { ... },
    "images": [
      {
        "data": "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAAB...",
        "format": "png",
        "filename": "chart_1.png",
        "page_number": 2,
        "dimensions": [640, 480],
        "colorspace": "RGB",
        "bits_per_component": 8,
        "is_mask": false,
        "description": "Chart showing quarterly results"
      }
    ],
    "image_ocr_results": [
      {
        "image": {
          "data": "data:image/jpeg;base64,/9j/4AAQSkZJRgABAQEAYABgAAD...",
          "format": "jpeg",
          "filename": "screenshot.jpg",
          "dimensions": [1024, 768]
        },
        "ocr_result": {
          "content": "Text extracted from image using OCR...",
          "mime_type": "text/plain",
          "metadata": { "quality_score": 0.95 }
        },
        "confidence_score": 0.87,
        "processing_time": 1.23,
        "skipped_reason": null
      }
    ]
  }
]
```

**Advanced Image OCR Configuration:**

For complex image OCR scenarios, use header-based configuration:

```bash
# Tesseract with multilingual support and custom PSM
curl -X POST http://localhost:8000/extract \
  -H "X-Extraction-Config: {
    \"extract_images\": true,
    \"ocr_extracted_images\": true,
    \"image_ocr_backend\": \"tesseract\",
    \"image_ocr_config\": {
      \"language\": \"eng+deu+fra\",
      \"psm\": 6,
      \"output_format\": \"text\"
    },
    \"deduplicate_images\": true,
    \"image_ocr_min_dimensions\": [200, 200],
    \"image_ocr_max_dimensions\": [4000, 4000]
  }" \
  -F "data=@multilingual_presentation.pptx"

# EasyOCR with confidence threshold and GPU acceleration
curl -X POST http://localhost:8000/extract \
  -H "X-Extraction-Config: {
    \"extract_images\": true,
    \"ocr_extracted_images\": true,
    \"image_ocr_backend\": \"easyocr\",
    \"image_ocr_config\": {
      \"language_list\": [\"en\", \"de\"],
      \"gpu\": false,
      \"confidence_threshold\": 0.6
    }
  }" \
  -F "data=@document_with_scene_text.pdf"
```

**Supported Document Types for Image Extraction:**

- **PDF documents**: Embedded images, graphics, and charts
- **PowerPoint presentations (PPTX)**: Slide images, shapes, and media
- **HTML documents**: Inline images and base64-encoded images
- **Microsoft Word documents (DOCX)**: Embedded images and charts
- **Email files (EML, MSG)**: Image attachments and inline images

Enable language detection:

```bash
curl -X POST "http://localhost:8000/extract?auto_detect_language=true" \
  -F "data=@multilingual_document.pdf"
```

**Supported Query Parameters:**

- `chunk_content` (boolean): Enable content chunking
- `max_chars` (integer): Maximum characters per chunk
- `max_overlap` (integer): Overlap between chunks in characters
- `extract_tables` (boolean): Enable table extraction
- `extract_entities` (boolean): Enable named entity extraction
- `extract_keywords` (boolean): Enable keyword extraction
- `keyword_count` (integer): Number of keywords to extract
- `force_ocr` (boolean): Force OCR processing
- `ocr_backend` (string): OCR engine (`tesseract`, `easyocr`, `paddleocr`)
- `auto_detect_language` (boolean): Enable automatic language detection
- `pdf_password` (string): Password for encrypted PDFs
- `extract_images` (boolean): Extract embedded images from supported formats (PDF, PPTX, HTML, Office, Email)
- `ocr_extracted_images` (boolean): Run OCR on extracted images to get text content
- `image_ocr_backend` (string): OCR engine to use for images (`tesseract`, `easyocr`, `paddleocr`)
- `image_ocr_min_width` / `image_ocr_min_height` (integer): Minimum image dimensions for OCR eligibility
- `image_ocr_max_width` / `image_ocr_max_height` (integer): Maximum image dimensions for OCR processing
- `deduplicate_images` (boolean): Remove duplicate images by content hash (enabled by default)

**Boolean Parameter Formats:**

Query parameters accept flexible boolean values:

- `true`, `false`
- `1`, `0`
- `yes`, `no`
- `on`, `off`

#### Header Configuration

For complex nested configurations, use the `X-Extraction-Config` header with JSON format:

Basic header configuration:

```bash
curl -X POST http://localhost:8000/extract \
  -H "X-Extraction-Config: {\"chunk_content\": true, \"max_chars\": 300, \"extract_keywords\": true}" \
  -F "data=@document.pdf"
```

Advanced OCR configuration:

```bash
curl -X POST http://localhost:8000/extract \
  -H "X-Extraction-Config: {
    \"force_ocr\": true,
    \"ocr_backend\": \"tesseract\",
    \"ocr_config\": {
      \"language\": \"eng+deu\",
      \"psm\": 6,
      \"output_format\": \"text\"
    }
  }" \
  -F "data=@multilingual_document.pdf"
```

Table extraction with GMFT configuration:

```bash
curl -X POST http://localhost:8000/extract \
  -H "X-Extraction-Config: {
    \"extract_tables\": true,
    \"gmft_config\": {
      \"detection_threshold\": 0.7,
      \"structure_threshold\": 0.5,
      \"detection_device\": \"auto\",
      \"structure_device\": \"auto\"
    }
  }" \
  -F "data=@document_with_tables.pdf"
```

#### Configuration Precedence

When multiple configuration sources are present, they are merged with the following precedence:

1. **Header config** (highest priority) - `X-Extraction-Config` header
1. **Query params** - URL query parameters
1. **Static config** - `kreuzberg.toml` or `pyproject.toml` files
1. **Defaults** (lowest priority) - Built-in default values

Header overrides query parameters:

```bash
curl -X POST "http://localhost:8000/extract?max_chars=1000" \
  -H "X-Extraction-Config: {\"max_chars\": 500}" \
  -F "data=@document.pdf"
```

Result: max_chars will be 500 (from header)

## Interactive API Documentation

Kreuzberg automatically generates comprehensive OpenAPI documentation that you can access through your web browser when the API server is running.

### Accessing the Documentation

Once the API server is running, you can access interactive documentation at:

- **OpenAPI Schema**: `http://localhost:8000/schema/openapi.json`
- **Swagger UI**: `http://localhost:8000/schema/swagger`
- **ReDoc Documentation**: `http://localhost:8000/schema/redoc`
- **Stoplight Elements**: `http://localhost:8000/schema/elements`
- **RapiDoc**: `http://localhost:8000/schema/rapidoc`

### Features

The interactive documentation provides:

- **Complete API Reference**: All endpoints with detailed parameter descriptions
- **Try It Out**: Test API endpoints directly from the browser
- **Request/Response Examples**: Sample requests and responses for each endpoint
- **Schema Validation**: Interactive validation of request parameters
- **Download Options**: Export the OpenAPI specification

### Example Usage

```bash
# Start the API server
litestar --app kreuzberg._api.main:app run

# Open your browser to view the documentation
open http://localhost:8000/schema/swagger
```

The documentation includes examples for all configuration options, making it easy to understand the full capabilities of the extraction API.

#### Error Handling

Invalid configuration returns appropriate error responses:

```bash
# Invalid JSON in header
curl -X POST http://localhost:8000/extract \
  -H "X-Extraction-Config: {invalid-json}" \
  -F "data=@document.pdf"

# Response: 400 Bad Request
{
  "message": "Invalid JSON in X-Extraction-Config header: ...",
  "details": "{\"error\": \"...\"}"
}
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

### Debugging 500 Errors

For detailed error information when 500 Internal Server Errors occur, set the `DEBUG` environment variable:

```bash
# Enable debug mode for detailed 500 error responses
DEBUG=1 litestar --app kreuzberg._api.main:app run

# Or with uvicorn
DEBUG=1 uvicorn kreuzberg._api.main:app --host 0.0.0.0 --port 8000
```

When `DEBUG=1` is set, 500 errors will include:

- Full stack traces
- Detailed error context
- Internal state information
- Request debugging details

⚠️ **Warning**: Only enable debug mode in development environments. Debug information may expose sensitive details and should never be used in production.

## Features

- **Runtime Configuration**: Configure extraction via query parameters and HTTP headers
- **Batch Processing**: Extract from multiple files in a single request
- **Automatic Format Detection**: Detects file types from MIME types
- **OCR Support**: Automatically applies OCR to images and scanned PDFs
- **Configuration Precedence**: Flexible configuration merging with clear precedence
- **Structured Logging**: Uses structlog for detailed logging
- **OpenTelemetry**: Built-in observability support
- **Async Processing**: High-performance async request handling

## Configuration

The API server uses the default Kreuzberg extraction configuration:

- Tesseract OCR is included by default
- PDF, image, and document extraction is supported
- Table extraction with GMFT (if installed)

### Environment Variables

The API server can be configured using environment variables for production deployments:

#### Server Configuration

| Variable                         | Description                  | Default            | Example            |
| -------------------------------- | ---------------------------- | ------------------ | ------------------ |
| `KREUZBERG_MAX_UPLOAD_SIZE`      | Maximum upload size in bytes | `1073741824` (1GB) | `2147483648` (2GB) |
| `KREUZBERG_ENABLE_OPENTELEMETRY` | Enable OpenTelemetry tracing | `true`             | `false`            |

#### Usage Examples

```bash
# Set 2GB upload limit
export KREUZBERG_MAX_UPLOAD_SIZE=2147483648
litestar --app kreuzberg._api.main:app run

# Disable telemetry
export KREUZBERG_ENABLE_OPENTELEMETRY=false
uvicorn kreuzberg._api.main:app --host 0.0.0.0 --port 8000

# Production settings with Docker
docker run -p 8000:8000 \
  -e KREUZBERG_MAX_UPLOAD_SIZE=5368709120 \
  -e KREUZBERG_ENABLE_OPENTELEMETRY=true \
  goldziher/kreuzberg:latest
```

**Note**: Boolean environment variables accept `true`/`false`, `1`/`0`, `yes`/`no`, or `on`/`off` values.

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
1. **Security**: Ensure `DEBUG` environment variable is not set

Example production command:

```bash
uvicorn kreuzberg._api.main:app \
  --host 0.0.0.0 \
  --port 8000 \
  --workers 4 \
  --log-level info
```
