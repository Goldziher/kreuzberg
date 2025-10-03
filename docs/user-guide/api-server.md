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

### Server Information

```bash
GET /info
```

Returns server version, configuration, and available backends:

```json
{
  "version": "4.0.0",
  "config": {
    "force_ocr": false,
    "ocr": null,
    "tables": null,
    "chunking": null
  },
  "cache_enabled": true,
  "available_backends": {
    "ocr": {
      "tesseract": true,
      "easyocr": false,
      "paddleocr": false
    },
    "features": {
      "vision_tables": true,
      "spacy": true
    }
  }
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
- Body:
    - `files`: One or more files to extract (required)
    - `config`: Optional JSON configuration (see examples below)
- **Maximum file size: Configurable via `KREUZBERG_MAX_UPLOAD_SIZE` environment variable (default: 1GB per file)**

**Response:**

- Status: 201 Created
- Body: Array of extraction results

**Basic Example:**

```bash
# Single file with default configuration
curl -F "files=@document.pdf" \
     http://localhost:8000/extract

# Multiple files
curl -F "files=@document1.pdf" \
     -F "files=@document2.docx" \
     -F "files=@image.jpg" \
     http://localhost:8000/extract
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
    "detected_languages": null,
    "tables": [],
    "images": []
  }
]
```

### Runtime Configuration

Configure extraction behavior by providing a JSON configuration in the `config` field:

#### Basic Configuration Examples

**Force OCR:**

```bash
curl -F "files=@image.jpg" \
     -F 'config={"force_ocr":true}' \
     http://localhost:8000/extract
```

**OCR with Tesseract:**

```bash
curl -F "files=@document.pdf" \
     -F 'config={"ocr":{"backend":"tesseract","language":"eng"}}' \
     http://localhost:8000/extract
```

**OCR with EasyOCR:**

```bash
curl -F "files=@document.jpg" \
     -F 'config={"ocr":{"backend":"easyocr","language":["en","de"]}}' \
     http://localhost:8000/extract
```

**OCR with PaddleOCR:**

```bash
curl -F "files=@chinese_document.jpg" \
     -F 'config={"ocr":{"backend":"paddleocr","language":"ch"}}' \
     http://localhost:8000/extract
```

#### Advanced OCR Configuration

**Tesseract with custom settings:**

```bash
curl -F "files=@multilingual_document.pdf" \
     -F 'config={
       "force_ocr": true,
       "ocr": {
         "backend": "tesseract",
         "language": "eng+deu",
         "psm": 6,
         "output_format": "text"
       }
     }' \
     http://localhost:8000/extract
```

**EasyOCR with device selection:**

```bash
curl -F "files=@document.jpg" \
     -F 'config={
       "ocr": {
         "backend": "easyocr",
         "language": ["en", "de"],
         "device": "cpu",
         "confidence_threshold": 0.6
       }
     }' \
     http://localhost:8000/extract
```

#### Chunking Configuration

**Enable chunking with custom settings:**

```bash
curl -F "files=@document.pdf" \
     -F 'config={"chunking":{"max_chars":500,"max_overlap":50}}' \
     http://localhost:8000/extract
```

#### Table Extraction

**Enable table extraction:**

```bash
curl -F "files=@document_with_tables.pdf" \
     -F 'config={"tables":{}}' \
     http://localhost:8000/extract
```

**Table extraction with custom thresholds:**

```bash
curl -F "files=@document_with_tables.pdf" \
     -F 'config={
       "tables": {
         "detection_threshold": 0.8,
         "structure_threshold": 0.6,
         "detection_device": "auto",
         "structure_device": "auto"
       }
     }' \
     http://localhost:8000/extract
```

#### Entity and Keyword Extraction

**Extract entities and keywords:**

```bash
curl -F "files=@document.pdf" \
     -F 'config={
       "entities": {},
       "keywords": {"top_k": 5}
     }' \
     http://localhost:8000/extract
```

**Entity extraction with language models:**

```bash
curl -F "files=@multilingual_document.pdf" \
     -F 'config={
       "language_detection": {},
       "entities": {
         "language_models": {
           "en": "en_core_web_sm",
           "de": "de_core_news_sm"
         },
         "fallback_to_multilingual": true
       }
     }' \
     http://localhost:8000/extract
```

#### Image Extraction

**Basic image extraction:**

```bash
curl -F "files=@document.pdf" \
     -F 'config={"images":{}}' \
     http://localhost:8000/extract
```

**Image extraction with OCR filtering:**

```bash
curl -F "files=@presentation.pptx" \
     -F 'config={
       "images": {
         "ocr_min_dimensions": [100, 100],
         "ocr_max_dimensions": [3000, 3000],
         "deduplicate": true
       }
     }' \
     http://localhost:8000/extract
```

**Image Extraction Response Format:**

When image extraction is enabled, the response includes additional fields:

```json
[
  {
    "content": "Main document text content...",
    "mime_type": "text/plain",
    "metadata": {},
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
    ]
  }
]
```

#### Language Detection

**Enable language detection:**

```bash
curl -F "files=@multilingual_document.pdf" \
     -F 'config={"language_detection":{}}' \
     http://localhost:8000/extract
```

**Advanced language detection:**

```bash
curl -F "files=@multilingual_document.pdf" \
     -F 'config={
       "language_detection": {
         "multilingual": true,
         "top_k": 3,
         "low_memory": false
       }
     }' \
     http://localhost:8000/extract
```

#### Combined Configuration

**Multiple features enabled:**

```bash
curl -F "files=@document.pdf" \
     -F 'config={
       "ocr": {
         "backend": "tesseract",
         "language": "eng"
       },
       "chunking": {
         "max_chars": 1000,
         "max_overlap": 200
       },
       "tables": {
         "detection_threshold": 0.7
       },
       "keywords": {
         "top_k": 10
       },
       "language_detection": {}
     }' \
     http://localhost:8000/extract
```

### Configuration Precedence

When multiple configuration sources are present, they are merged with the following precedence:

1. **Request config** (highest priority) - `config` field in request
1. **Static config** - `kreuzberg.toml` or `pyproject.toml` files
1. **Defaults** (lowest priority) - Built-in default values

### Cache Management

#### Get Cache Statistics

```bash
GET /cache/stats?type=all
```

Query parameters:

- `type`: Cache type (`document`, `ocr`, `mime`, `table`, `all`) - default: `all`

Returns statistics for the specified cache(s):

```json
{
  "document": {
    "hits": 150,
    "misses": 50,
    "size": 1024000,
    "max_size": 10485760
  },
  "ocr": {
    "hits": 75,
    "misses": 25,
    "size": 512000,
    "max_size": 5242880
  }
}
```

#### Clear Cache

```bash
DELETE /cache?type=all
```

Query parameters:

- `type`: Cache type (`document`, `ocr`, `mime`, `table`, `all`) - default: `all`

Returns confirmation:

```json
{
  "message": "Cache cleared successfully",
  "type": "all"
}
```

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

## Error Handling

The API uses standard HTTP status codes:

- `200 OK`: Successful health check or cache operation
- `201 Created`: Successful extraction
- `400 Bad Request`: Validation error (e.g., invalid configuration)
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

## Environment Variables

The API server can be configured using environment variables for production deployments:

### Server Configuration

| Variable                         | Description                  | Default            | Example            |
| -------------------------------- | ---------------------------- | ------------------ | ------------------ |
| `KREUZBERG_MAX_UPLOAD_SIZE`      | Maximum upload size in bytes | `1073741824` (1GB) | `2147483648` (2GB) |
| `KREUZBERG_ENABLE_OPENTELEMETRY` | Enable OpenTelemetry tracing | `true`             | `false`            |
| `KREUZBERG_CACHE_ENABLED`        | Enable caching               | `true`             | `false`            |
| `DEBUG`                          | Enable debug mode            | `false`            | `true`             |

### Cache Configuration

| Variable                   | Description             | Default     | Example      |
| -------------------------- | ----------------------- | ----------- | ------------ |
| `KREUZBERG_CACHE_DIR`      | Cache directory         | System temp | `/tmp/cache` |
| `KREUZBERG_CACHE_MAX_SIZE` | Max cache size in bytes | `10485760`  | `52428800`   |
| `KREUZBERG_CACHE_TTL`      | Cache TTL in seconds    | `3600`      | `7200`       |

### Usage Examples

```bash
# Set 2GB upload limit
export KREUZBERG_MAX_UPLOAD_SIZE=2147483648
litestar --app kreuzberg._api.main:app run

# Disable telemetry and caching
export KREUZBERG_ENABLE_OPENTELEMETRY=false
export KREUZBERG_CACHE_ENABLED=false
uvicorn kreuzberg._api.main:app --host 0.0.0.0 --port 8000

# Production settings with Docker
docker run -p 8000:8000 \
  -e KREUZBERG_MAX_UPLOAD_SIZE=5368709120 \
  -e KREUZBERG_ENABLE_OPENTELEMETRY=true \
  -e KREUZBERG_CACHE_ENABLED=true \
  goldziher/kreuzberg:latest
```

**Note**: Boolean environment variables accept `true`/`false`, `1`/`0`, `yes`/`no`, or `on`/`off` values.

## Features

- **Runtime Configuration**: Configure extraction via JSON in multipart form data
- **Batch Processing**: Extract from multiple files in a single request
- **Automatic Format Detection**: Detects file types from MIME types
- **OCR Support**: Multiple OCR backends (Tesseract, EasyOCR, PaddleOCR)
- **Table Extraction**: Vision-based table detection and extraction
- **Entity & Keyword Extraction**: Named entity recognition and keyword extraction
- **Image Extraction**: Extract embedded images with optional OCR
- **Language Detection**: Automatic language identification
- **Caching**: High-performance caching with statistics and management
- **Structured Logging**: Uses structlog for detailed logging
- **OpenTelemetry**: Built-in observability support
- **Async Processing**: High-performance async request handling

## Production Deployment

For production use, consider:

1. **Reverse Proxy**: Use nginx or similar for SSL termination
1. **Process Manager**: Use systemd, supervisor, or similar
1. **Workers**: Run multiple workers with uvicorn or gunicorn
1. **Monitoring**: Enable OpenTelemetry exporters
1. **Rate Limiting**: Add rate limiting middleware
1. **Authentication**: Add authentication middleware if needed
1. **Security**: Ensure `DEBUG` environment variable is not set
1. **Caching**: Configure appropriate cache size and TTL for your workload

Example production command:

```bash
uvicorn kreuzberg._api.main:app \
  --host 0.0.0.0 \
  --port 8000 \
  --workers 4 \
  --log-level info
```

## Static Configuration

The API server automatically loads configuration from `kreuzberg.toml` or `pyproject.toml` if present. This provides default extraction settings that can be overridden per-request.

Example `kreuzberg.toml`:

```toml
force_ocr = false

[ocr]
backend = "tesseract"
language = "eng"
psm = 6

[chunking]
max_chars = 1000
max_overlap = 200

[tables]
detection_threshold = 0.7

[keywords]
top_k = 10
```

See the [Extraction Configuration](extraction-configuration.md) guide for complete configuration file documentation.
