# API Server

Kreuzberg provides two server modes for programmatic access: an HTTP REST API server for general integration and a Model Context Protocol (MCP) server for AI agent integration.

## Server Types

### HTTP REST API Server

A production-ready HTTP API server providing RESTful endpoints for document extraction, health checks, and cache management.

**Best for:**
- Web applications
- Microservices integration
- General HTTP clients
- Load-balanced deployments

### MCP Server

A Model Context Protocol server that exposes Kreuzberg as tools for AI agents and assistants.

**Best for:**
- AI agent integration (Claude, GPT, etc.)
- Agentic workflows
- Tool use by language models
- Stdio-based communication

## HTTP REST API

### Starting the Server

=== "CLI"

    --8<-- "snippets/api_server/cli.md"

=== "C#"

    --8<-- "snippets/api_server/csharp.md"

=== "Docker"

    --8<-- "snippets/api_server/docker.md"

=== "Go"

    --8<-- "snippets/api_server/go.md"

=== "Java"

    --8<-- "snippets/api_server/java.md"

=== "Python"

    --8<-- "snippets/api_server/python.md"

=== "Rust"

    --8<-- "snippets/api_server/rust.md"

### API Endpoints

#### POST /extract

Extract text from uploaded files via multipart form data.

**Request Format:**

- **Method:** POST
- **Content-Type:** `multipart/form-data`
- **Fields:**
    - `files` (required, repeatable): Files to extract
    - `config` (optional): JSON configuration overrides

**Response:** JSON array of extraction results

**Example:**

```bash
# Single file
curl -F "files=@document.pdf" http://localhost:8000/extract

# Multiple files
curl -F "files=@doc1.pdf" -F "files=@doc2.docx" \
  http://localhost:8000/extract

# With configuration override
curl -F "files=@scanned.pdf" \
     -F 'config={"ocr":{"language":"eng"},"force_ocr":true}' \
  http://localhost:8000/extract
```

**Response Schema:**

```json
[
  {
    "content": "Extracted text content...",
    "mime_type": "application/pdf",
    "metadata": {
      "page_count": 10,
      "author": "John Doe"
    },
    "tables": [],
    "detected_languages": ["eng"],
    "chunks": null,
    "images": null
  }
]
```

#### GET /health

Health check endpoint for monitoring and load balancers.

**Example:**

```bash
curl http://localhost:8000/health
```

**Response:**

```json
{
  "status": "healthy",
  "version": "4.0.0-rc.1"
}
```

#### GET /info

Server information and capabilities.

**Example:**

```bash
curl http://localhost:8000/info
```

**Response:**

```json
{
  "version": "4.0.0-rc.1",
  "rust_backend": true
}
```

#### GET /cache/stats

Get cache statistics.

**Example:**

```bash
curl http://localhost:8000/cache/stats
```

**Response:**

```json
{
  "directory": "/home/user/.cache/kreuzberg",
  "total_files": 42,
  "total_size_mb": 156.8,
  "available_space_mb": 45123.5,
  "oldest_file_age_days": 7.2,
  "newest_file_age_days": 0.1
}
```

#### DELETE /cache/clear

Clear all cached files.

**Example:**

```bash
curl -X DELETE http://localhost:8000/cache/clear
```

**Response:**

```json
{
  "directory": "/home/user/.cache/kreuzberg",
  "removed_files": 42,
  "freed_mb": 156.8
}
```

### Configuration

#### Configuration File Discovery

The server automatically discovers configuration files in this order:

1. `./kreuzberg.toml` (current directory)
2. `./kreuzberg.yaml`
3. `./kreuzberg.json`
4. Parent directories (recursive search)
5. Default configuration (if no file found)

**Example kreuzberg.toml:**

```toml
# OCR settings
[ocr]
backend = "tesseract"
language = "eng"

# Features
enable_quality_processing = true
use_cache = true

# Token reduction
[token_reduction]
enabled = true
target_reduction = 0.3
```

See [Configuration Guide](configuration.md) for all options.

#### Environment Variables

**Server Binding:**

```bash
KREUZBERG_HOST=0.0.0.0          # Listen address (default: 127.0.0.1)
KREUZBERG_PORT=8000              # Port number (default: 8000)
```

**Upload Limits:**

```bash
KREUZBERG_MAX_UPLOAD_SIZE_MB=200  # Max upload size in MB (default: 100)
```

**CORS Configuration:**

```bash
# Comma-separated list of allowed origins
KREUZBERG_CORS_ORIGINS="https://app.example.com,https://api.example.com"
```

**Security Warning:** The default CORS configuration allows all origins for development convenience. This permits CSRF attacks. Always set `KREUZBERG_CORS_ORIGINS` in production.

### Client Examples

=== "C#"

    --8<-- "snippets/csharp/client_extract_single_file.md"

=== "cURL"

    ```bash
    # Extract single file
    curl -F "files=@document.pdf" http://localhost:8000/extract | jq .

    # Extract with OCR
    curl -F "files=@scanned.pdf" \
         -F 'config={"ocr":{"language":"eng"}}' \
         http://localhost:8000/extract | jq .

    # Multiple files
    curl -F "files=@doc1.pdf" \
         -F "files=@doc2.docx" \
         http://localhost:8000/extract | jq .
    ```

=== "Go"

    --8<-- "snippets/go/client_extract_single_file.md"

=== "Java"

    --8<-- "snippets/java/client_extract_single_file.md"

=== "Python"

    --8<-- "snippets/python/client_extract_single_file.md"

=== "Ruby"

    --8<-- "snippets/ruby/client_extract_single_file.md"

=== "Rust"

    --8<-- "snippets/rust/client_extract_single_file.md"

=== "TypeScript"

    --8<-- "snippets/typescript/client_extract_single_file.md"

### Error Handling

**Error Response Format:**

```json
{
  "error_type": "ValidationError",
  "message": "Invalid file format",
  "traceback": "...",
  "status_code": 400
}
```

**HTTP Status Codes:**

| Status Code | Error Type | Meaning |
|------------|------------|---------|
| 400 | `ValidationError` | Invalid input parameters |
| 422 | `ParsingError`, `OcrError` | Document processing failed |
| 500 | Internal errors | Server errors |

**Example:**

=== "C#"

    --8<-- "snippets/csharp/error_handling_extract.md"

=== "Go"

    --8<-- "snippets/go/error_handling_extract.md"

=== "Java"

    --8<-- "snippets/java/error_handling_extract.md"

=== "Python"

    --8<-- "snippets/python/error_handling_extract.md"

=== "Ruby"

    --8<-- "snippets/ruby/error_handling_extract.md"

=== "Rust"

    --8<-- "snippets/rust/error_handling_extract.md"

=== "TypeScript"

    --8<-- "snippets/typescript/error_handling_extract.md"

## MCP Server

The Model Context Protocol (MCP) server exposes Kreuzberg as tools for AI agents and assistants.

### Starting the MCP Server

=== "CLI"

    ```bash
    # Start MCP server (stdio transport)
    kreuzberg mcp

    # With configuration file
    kreuzberg mcp --config kreuzberg.toml
    ```

=== "C#"

    --8<-- "snippets/csharp/mcp_server_start.md"

=== "Go"

    --8<-- "snippets/go/mcp_server_start.md"

=== "Java"

    --8<-- "snippets/java/mcp_server_start.md"

=== "Python"

    --8<-- "snippets/python/mcp_server_start.md"

=== "Ruby"

    --8<-- "snippets/ruby/mcp_server_start.md"

=== "Rust"

    --8<-- "snippets/rust/mcp_server_start.md"

=== "TypeScript"

    --8<-- "snippets/typescript/mcp_server_start.md"

### MCP Tools

The MCP server exposes 6 tools for AI agents:

#### extract_file

Extract content from a file path.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `path` | string | Yes | File path to extract |
| `mime_type` | string | No | MIME type hint |
| `enable_ocr` | boolean | No | Enable OCR (default: false) |
| `force_ocr` | boolean | No | Force OCR even if text exists (default: false) |
| `async` | boolean | No | Use async extraction (default: true) |

**Example MCP Request:**

```json
{
  "method": "tools/call",
  "params": {
    "name": "extract_file",
    "arguments": {
      "path": "/path/to/document.pdf",
      "enable_ocr": true,
      "async": true
    }
  }
}
```

#### extract_bytes

Extract content from base64-encoded file data.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `data` | string | Yes | Base64-encoded file content |
| `mime_type` | string | No | MIME type hint |
| `enable_ocr` | boolean | No | Enable OCR |
| `force_ocr` | boolean | No | Force OCR |
| `async` | boolean | No | Use async extraction |

#### batch_extract_files

Extract multiple files in parallel.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `paths` | array[string] | Yes | File paths to extract |
| `enable_ocr` | boolean | No | Enable OCR |
| `force_ocr` | boolean | No | Force OCR |
| `async` | boolean | No | Use async extraction |

#### detect_mime_type

Detect file format and return MIME type.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `path` | string | Yes | File path |
| `use_content` | boolean | No | Content-based detection (default: true) |

#### cache_stats

Get cache statistics.

**Parameters:** None

**Returns:** Cache directory path, file count, size, available space, file ages

#### cache_clear

Clear all cached files.

**Parameters:** None

**Returns:** Number of files removed, space freed

### MCP Server Information

**Server Metadata:**

- **Name:** `kreuzberg-mcp`
- **Title:** Kreuzberg Document Intelligence MCP Server
- **Version:** Current package version
- **Website:** https://goldziher.github.io/kreuzberg/
- **Protocol:** MCP (Model Context Protocol)
- **Transport:** stdio (stdin/stdout)

**Capabilities:**

- Tool calling (6 tools exposed)
- Async and sync extraction variants
- Base64-encoded file handling
- Batch processing

### AI Agent Integration

=== "Claude Desktop"

    Add to Claude Desktop configuration (`~/Library/Application Support/Claude/claude_desktop_config.json` on macOS):

    ```json
    {
      "mcpServers": {
        "kreuzberg": {
          "command": "kreuzberg",
          "args": ["mcp"]
        }
      }
    }
    ```

=== "C#"

    --8<-- "snippets/csharp/mcp_custom_client.md"

=== "Go"

    --8<-- "snippets/go/mcp_custom_client.md"

=== "Java"

    --8<-- "snippets/java/mcp_client.md"

=== "LangChain"

    --8<-- "snippets/python/mcp_langchain_integration.md"

=== "Python"

    --8<-- "snippets/python/mcp_custom_client.md"

=== "Ruby"

    --8<-- "snippets/ruby/mcp_custom_client.md"

=== "Rust"

    --8<-- "snippets/rust/mcp_custom_client.md"

=== "TypeScript"

    --8<-- "snippets/typescript/mcp_custom_client.md"

## Production Deployment

### Docker Deployment

**Docker Compose Example:**

```yaml
version: '3.8'

services:
  kreuzberg-api:
    image: goldziher/kreuzberg:latest
    ports:
      - "8000:8000"
    environment:
      - KREUZBERG_CORS_ORIGINS=https://myapp.com,https://api.myapp.com
      - KREUZBERG_MAX_UPLOAD_SIZE_MB=500
    volumes:
      - ./config:/config
      - ./cache:/root/.cache/kreuzberg
    command: serve -H 0.0.0.0 -p 8000 --config /config/kreuzberg.toml
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8000/health"]
      interval: 30s
      timeout: 10s
      retries: 3
```

**Run:**

```bash
docker-compose up -d
```

### Kubernetes Deployment

**Deployment Manifest:**

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: kreuzberg-api
spec:
  replicas: 3
  selector:
    matchLabels:
      app: kreuzberg-api
  template:
    metadata:
      labels:
        app: kreuzberg-api
    spec:
      containers:
      - name: kreuzberg
        image: goldziher/kreuzberg:latest
        ports:
        - containerPort: 8000
        env:
        - name: KREUZBERG_CORS_ORIGINS
          value: "https://myapp.com"
        - name: KREUZBERG_MAX_UPLOAD_SIZE_MB
          value: "500"
        command: ["kreuzberg", "serve", "-H", "0.0.0.0", "-p", "8000"]
        livenessProbe:
          httpGet:
            path: /health
            port: 8000
          initialDelaySeconds: 10
          periodSeconds: 30
        readinessProbe:
          httpGet:
            path: /health
            port: 8000
          initialDelaySeconds: 5
          periodSeconds: 10
        resources:
          requests:
            memory: "512Mi"
            cpu: "500m"
          limits:
            memory: "2Gi"
            cpu: "2000m"
---
apiVersion: v1
kind: Service
metadata:
  name: kreuzberg-api
spec:
  selector:
    app: kreuzberg-api
  ports:
  - protocol: TCP
    port: 80
    targetPort: 8000
  type: LoadBalancer
```

### Reverse Proxy Configuration

**Nginx:**

```nginx
upstream kreuzberg {
    server 127.0.0.1:8000;
    server 127.0.0.1:8001;
    server 127.0.0.1:8002;
}

server {
    listen 443 ssl http2;
    server_name api.example.com;

    ssl_certificate /path/to/cert.pem;
    ssl_certificate_key /path/to/key.pem;

    # Increase upload size limit
    client_max_body_size 500M;

    location / {
        proxy_pass http://kreuzberg;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;

        # Timeouts for large files
        proxy_read_timeout 300s;
        proxy_send_timeout 300s;
    }

    location /health {
        proxy_pass http://kreuzberg;
        access_log off;
    }
}
```

**Caddy:**

```caddy
api.example.com {
    reverse_proxy localhost:8000 localhost:8001 localhost:8002 {
        lb_policy round_robin
        health_uri /health
        health_interval 10s
    }

    # Increase upload size
    request_body {
        max_size 500MB
    }
}
```

### Production Checklist

1. Set `KREUZBERG_CORS_ORIGINS` to explicit allowed origins
2. Configure `KREUZBERG_MAX_UPLOAD_SIZE_MB` based on expected document sizes
3. Use reverse proxy (Nginx/Caddy) for SSL/TLS termination
4. Enable logging via `RUST_LOG=info` environment variable
5. Set up health checks on `/health` endpoint
6. Monitor cache size and set up periodic clearing
7. Use `0.0.0.0` binding for containerized deployments
8. Configure resource limits (CPU, memory) in container orchestration
9. Test with large files to validate upload limits and timeouts
10. Implement rate limiting at reverse proxy level
11. Set up monitoring (Prometheus metrics, logs aggregation)
12. Plan for horizontal scaling with load balancing

### Monitoring

**Health Check Endpoint:**

```bash
# Simple check
curl http://localhost:8000/health

# With monitoring script
#!/bin/bash
while true; do
  if curl -f http://localhost:8000/health > /dev/null 2>&1; then
    echo "$(date): Server healthy"
  else
    echo "$(date): Server unhealthy"
    # Send alert
  fi
  sleep 30
done
```

**Cache Monitoring:**

```bash
# Check cache size
curl http://localhost:8000/cache/stats | jq .

# Clear cache if too large
CACHE_SIZE=$(curl -s http://localhost:8000/cache/stats | jq .total_size_mb)
if (( $(echo "$CACHE_SIZE > 1000" | bc -l) )); then
  curl -X DELETE http://localhost:8000/cache/clear
fi
```

**Logging:**

```bash
# Run with debug logging
RUST_LOG=debug kreuzberg serve -H 0.0.0.0 -p 8000

# Production logging (info level)
RUST_LOG=info kreuzberg serve -H 0.0.0.0 -p 8000

# JSON structured logging
RUST_LOG=info RUST_LOG_FORMAT=json kreuzberg serve -H 0.0.0.0 -p 8000
```

## Performance Tuning

### Upload Size Limits

Configure based on expected document sizes:

```bash
# For small documents (< 10 MB)
export KREUZBERG_MAX_UPLOAD_SIZE_MB=50

# For typical documents (< 50 MB)
export KREUZBERG_MAX_UPLOAD_SIZE_MB=200

# For large scans and archives
export KREUZBERG_MAX_UPLOAD_SIZE_MB=1000
```

### Concurrent Requests

The server handles concurrent requests efficiently using Tokio's async runtime. For high-throughput scenarios:

1. **Run multiple instances** behind a load balancer
2. **Configure reverse proxy connection pooling**
3. **Monitor CPU and memory usage** to determine optimal replica count

### Cache Strategy

Configure cache behavior via `kreuzberg.toml`:

```toml
use_cache = true
cache_dir = "/var/cache/kreuzberg"  # Custom cache location
```

**Cache clearing strategies:**

```bash
# Periodic clearing (cron job)
0 2 * * * curl -X DELETE http://localhost:8000/cache/clear

# Size-based clearing
CACHE_SIZE=$(curl -s http://localhost:8000/cache/stats | jq .total_size_mb)
if [ "$CACHE_SIZE" -gt 1000 ]; then
  curl -X DELETE http://localhost:8000/cache/clear
fi
```

## Next Steps

- [Configuration Guide](configuration.md) - Detailed configuration options
- [CLI Usage](../cli/usage.md) - Command-line interface
- [Advanced Features](advanced.md) - Chunking, language detection, token reduction
- [Plugin Development](plugins.md) - Extend Kreuzberg functionality
