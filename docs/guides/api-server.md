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

```bash title="Terminal"
# Extract a single file via HTTP POST
curl -F "files=@document.pdf" http://localhost:8000/extract

# Extract multiple files in a single request
curl -F "files=@doc1.pdf" -F "files=@doc2.docx" \
  http://localhost:8000/extract

# Extract with custom OCR configuration override
curl -F "files=@scanned.pdf" \
     -F 'config={"ocr":{"language":"eng"},"force_ocr":true}' \
  http://localhost:8000/extract
```

**Response Schema:**

```json title="Response"
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

```bash title="Terminal"
# Check server health status
curl http://localhost:8000/health
```

**Response:**

```json title="Response"
{
  "status": "healthy",
  "version": "4.0.0-rc.1"
}
```

#### GET /info

Server information and capabilities.

**Example:**

```bash title="Terminal"
# Get server version and capabilities
curl http://localhost:8000/info
```

**Response:**

```json title="Response"
{
  "version": "4.0.0-rc.1",
  "rust_backend": true
}
```

#### GET /cache/stats

Get cache statistics.

**Example:**

```bash title="Terminal"
# Retrieve cache statistics and storage usage
curl http://localhost:8000/cache/stats
```

**Response:**

```json title="Response"
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

```bash title="Terminal"
# Clear all cached extraction results
curl -X DELETE http://localhost:8000/cache/clear
```

**Response:**

```json title="Response"
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

```toml title="kreuzberg.toml"
# Configure OCR backend and language settings
[ocr]
backend = "tesseract"
language = "eng"

# Enable quality processing and caching
enable_quality_processing = true
use_cache = true

# Configure token reduction for LLM optimization
[token_reduction]
enabled = true
target_reduction = 0.3
```

See [Configuration Guide](configuration.md) for all options.

#### Environment Variables

**Server Binding:**

```bash title="Terminal"
# Configure server listen address and port
KREUZBERG_HOST=0.0.0.0          # Listen address (default: 127.0.0.1)
KREUZBERG_PORT=8000              # Port number (default: 8000)
```

**Upload Limits:**

```bash title="Terminal"
# Set maximum file upload size in megabytes
KREUZBERG_MAX_UPLOAD_SIZE_MB=200  # Max upload size in MB (default: 100)
```

**CORS Configuration:**

```bash title="Terminal"
# Configure allowed origins for cross-origin requests (production security)
KREUZBERG_CORS_ORIGINS="https://app.example.com,https://api.example.com"
```

**Security Warning:** The default CORS configuration allows all origins for development convenience. This permits CSRF attacks. Always set `KREUZBERG_CORS_ORIGINS` in production.

### Client Examples

=== "C#"

    --8<-- "snippets/csharp/client_extract_single_file.md"

=== "cURL"

    ```bash title="Terminal"
    # Extract content from a single document
    curl -F "files=@document.pdf" http://localhost:8000/extract | jq .

    # Extract with OCR enabled for scanned documents
    curl -F "files=@scanned.pdf" \
         -F 'config={"ocr":{"language":"eng"}}' \
         http://localhost:8000/extract | jq .

    # Batch extract multiple files in parallel
    curl -F "files=@doc1.pdf" \
         -F "files=@doc2.docx" \
         http://localhost:8000/extract | jq .
    ```

=== "Go"

    --8<-- "snippets/go/api/client_extract_single_file.md"

=== "Java"

    --8<-- "snippets/java/api/client_extract_single_file.md"

=== "Python"

    --8<-- "snippets/python/api/client_extract_single_file.md"

=== "Ruby"

    --8<-- "snippets/ruby/api/client_extract_single_file.md"

=== "Rust"

    --8<-- "snippets/rust/api/client_extract_single_file.md"

=== "TypeScript"

    --8<-- "snippets/typescript/getting-started/client_extract_single_file.md"

### Error Handling

**Error Response Format:**

```json title="Error Response"
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

    --8<-- "snippets/go/api/error_handling_extract.md"

=== "Java"

    --8<-- "snippets/java/api/error_handling_extract.md"

=== "Python"

    --8<-- "snippets/python/utils/error_handling_extract.md"

=== "Ruby"

    --8<-- "snippets/ruby/api/error_handling_extract.md"

=== "Rust"

    --8<-- "snippets/rust/api/error_handling_extract.md"

=== "TypeScript"

    --8<-- "snippets/typescript/api/error_handling_extract.md"

## MCP Server

The Model Context Protocol (MCP) server exposes Kreuzberg as tools for AI agents and assistants.

### Starting the MCP Server

=== "CLI"

    ```bash title="Terminal"
    # Start MCP server using stdio transport for AI agents
    kreuzberg mcp

    # Start MCP server with custom configuration file
    kreuzberg mcp --config kreuzberg.toml
    ```

=== "C#"

    --8<-- "snippets/csharp/mcp_server_start.md"

=== "Go"

    --8<-- "snippets/go/mcp/mcp_server_start.md"

=== "Java"

    --8<-- "snippets/java/mcp/mcp_server_start.md"

=== "Python"

    --8<-- "snippets/python/mcp/mcp_server_start.md"

=== "Ruby"

    --8<-- "snippets/ruby/mcp/mcp_server_start.md"

=== "Rust"

    --8<-- "snippets/rust/mcp/mcp_server_start.md"

=== "TypeScript"

    --8<-- "snippets/typescript/mcp/mcp_server_start.md"

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

```json title="MCP Request"
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

    ```json title="claude_desktop_config.json"
    {
      "mcpServers": {
        "kreuzberg": {
          "command": "kreuzberg",
          "args": ["mcp"]
        }
      }
    }
    ```

    After adding the configuration, restart Claude Desktop to load the Kreuzberg MCP server.

=== "C#"

    --8<-- "snippets/csharp/mcp_custom_client.md"

=== "Go"

    --8<-- "snippets/go/mcp/mcp_custom_client.md"

=== "Java"

    --8<-- "snippets/java/mcp/mcp_client.md"

=== "LangChain"

    --8<-- "snippets/python/mcp/mcp_langchain_integration.md"

=== "Python"

    --8<-- "snippets/python/mcp/mcp_custom_client.md"

=== "Ruby"

    --8<-- "snippets/ruby/mcp/mcp_custom_client.md"

=== "Rust"

    --8<-- "snippets/rust/mcp/mcp_custom_client.md"

=== "TypeScript"

    --8<-- "snippets/typescript/mcp/mcp_custom_client.md"

## Production Deployment

### Docker Deployment

**Docker Compose Example:**

```yaml title="docker-compose.yaml"
version: '3.8'

services:
  kreuzberg-api:
    image: goldziher/kreuzberg:latest
    ports:
      - "8000:8000"
    environment:
      # Configure CORS for production security
      - KREUZBERG_CORS_ORIGINS=https://myapp.com,https://api.myapp.com
      # Set maximum upload size for large documents
      - KREUZBERG_MAX_UPLOAD_SIZE_MB=500
    volumes:
      # Mount configuration and cache directories
      - ./config:/config
      - ./cache:/root/.cache/kreuzberg
    command: serve -H 0.0.0.0 -p 8000 --config /config/kreuzberg.toml
    restart: unless-stopped
    healthcheck:
      # Health check for container orchestration
      test: ["CMD", "curl", "-f", "http://localhost:8000/health"]
      interval: 30s
      timeout: 10s
      retries: 3
```

**Run:**

```bash title="Terminal"
# Start the Kreuzberg API server in detached mode
docker-compose up -d
```

### Kubernetes Deployment

**Deployment Manifest:**

```yaml title="kubernetes-deployment.yaml"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: kreuzberg-api
spec:
  replicas: 3  # Deploy 3 replicas for high availability
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
        # Production environment configuration
        - name: KREUZBERG_CORS_ORIGINS
          value: "https://myapp.com"
        - name: KREUZBERG_MAX_UPLOAD_SIZE_MB
          value: "500"
        command: ["kreuzberg", "serve", "-H", "0.0.0.0", "-p", "8000"]
        livenessProbe:
          # Check if container is alive and healthy
          httpGet:
            path: /health
            port: 8000
          initialDelaySeconds: 10
          periodSeconds: 30
        readinessProbe:
          # Check if container is ready to accept traffic
          httpGet:
            path: /health
            port: 8000
          initialDelaySeconds: 5
          periodSeconds: 10
        resources:
          # Resource limits for optimal performance
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
  type: LoadBalancer  # Expose service via load balancer
```

### Reverse Proxy Configuration

**Nginx:**

```nginx title="nginx.conf"
# Load balance across multiple Kreuzberg instances
upstream kreuzberg {
    server 127.0.0.1:8000;
    server 127.0.0.1:8001;
    server 127.0.0.1:8002;
}

server {
    listen 443 ssl http2;
    server_name api.example.com;

    # SSL/TLS configuration
    ssl_certificate /path/to/cert.pem;
    ssl_certificate_key /path/to/key.pem;

    # Increase upload size limit for large documents
    client_max_body_size 500M;

    location / {
        proxy_pass http://kreuzberg;
        # Forward client headers
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;

        # Extended timeouts for large file processing
        proxy_read_timeout 300s;
        proxy_send_timeout 300s;
    }

    location /health {
        proxy_pass http://kreuzberg;
        access_log off;  # Disable logging for health checks
    }
}
```

**Caddy:**

```caddy title="Caddyfile"
api.example.com {
    # Load balance with automatic health checks
    reverse_proxy localhost:8000 localhost:8001 localhost:8002 {
        lb_policy round_robin
        health_uri /health
        health_interval 10s
    }

    # Increase maximum upload size for large documents
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

```bash title="Terminal"
# Simple health check for manual verification
curl http://localhost:8000/health

# Continuous monitoring script for production
#!/bin/bash
while true; do
  if curl -f http://localhost:8000/health > /dev/null 2>&1; then
    echo "$(date): Server healthy"
  else
    echo "$(date): Server unhealthy"
    # Send alert to monitoring system
  fi
  sleep 30
done
```

**Cache Monitoring:**

```bash title="Terminal"
# Retrieve cache statistics and usage metrics
curl http://localhost:8000/cache/stats | jq .

# Automatic cache clearing when size exceeds threshold
CACHE_SIZE=$(curl -s http://localhost:8000/cache/stats | jq .total_size_mb)
if (( $(echo "$CACHE_SIZE > 1000" | bc -l) )); then
  curl -X DELETE http://localhost:8000/cache/clear
fi
```

**Logging:**

```bash title="Terminal"
# Run with debug logging for development and troubleshooting
RUST_LOG=debug kreuzberg serve -H 0.0.0.0 -p 8000

# Production logging with info level (recommended)
RUST_LOG=info kreuzberg serve -H 0.0.0.0 -p 8000

# JSON structured logging for log aggregation systems
RUST_LOG=info RUST_LOG_FORMAT=json kreuzberg serve -H 0.0.0.0 -p 8000
```

## Performance Tuning

### Upload Size Limits

Configure based on expected document sizes:

```bash title="Terminal"
# Configuration for small documents (PDFs, images under 10 MB)
export KREUZBERG_MAX_UPLOAD_SIZE_MB=50

# Configuration for typical business documents (under 50 MB)
export KREUZBERG_MAX_UPLOAD_SIZE_MB=200

# Configuration for large scans, archives, and high-resolution images
export KREUZBERG_MAX_UPLOAD_SIZE_MB=1000
```

### Concurrent Requests

The server handles concurrent requests efficiently using Tokio's async runtime. For high-throughput scenarios:

1. **Run multiple instances** behind a load balancer
2. **Configure reverse proxy connection pooling**
3. **Monitor CPU and memory usage** to determine optimal replica count

### Cache Strategy

Configure cache behavior via `kreuzberg.toml`:

```toml title="kreuzberg.toml"
# Enable caching for faster repeated extractions
use_cache = true
cache_dir = "/var/cache/kreuzberg"  # Custom cache location for production
```

**Cache clearing strategies:**

```bash title="Terminal"
# Periodic cache clearing via cron job (daily at 2 AM)
0 2 * * * curl -X DELETE http://localhost:8000/cache/clear

# Size-based cache clearing when threshold is exceeded
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
