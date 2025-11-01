# API Server

Kreuzberg provides two server modes for programmatic access: an HTTP REST API server for general integration and a Model Context Protocol (MCP) server for AI agent integration.

## Server Types

### HTTP REST API Server

A production-ready HTTP API server built with Axum (Rust) providing RESTful endpoints for document extraction, health checks, and cache management.

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

    ```bash
    # Default: http://127.0.0.1:8000
    kreuzberg serve

    # Custom host and port
    kreuzberg serve -H 0.0.0.0 -p 3000

    # With configuration file
    kreuzberg serve --config kreuzberg.toml
    ```

=== "Python"

    ```python
    import subprocess

    # Start server
    subprocess.Popen(["python", "-m", "kreuzberg", "serve", "-H", "0.0.0.0", "-p", "8000"])

    # Or use the CLI module
    from kreuzberg.__main__ import main
    import sys

    sys.argv = ["kreuzberg", "serve", "-H", "0.0.0.0", "-p", "8000"]
    main()
    ```

=== "Rust"

    ```rust
    use kreuzberg::{ExtractionConfig, api::serve_with_config};

    #[tokio::main]
    async fn main() -> kreuzberg::Result<()> {
        let config = ExtractionConfig::discover()?;
        serve_with_config("0.0.0.0", 8000, config).await?;
        Ok(())
    }
    ```

=== "Docker"

    ```bash
    # Run server on port 8000
    docker run -d \
      -p 8000:8000 \
      goldziher/kreuzberg:latest \
      serve -H 0.0.0.0 -p 8000

    # With environment variables
    docker run -d \
      -e KREUZBERG_CORS_ORIGINS="https://myapp.com" \
      -e KREUZBERG_MAX_UPLOAD_SIZE_MB=200 \
      -p 8000:8000 \
      goldziher/kreuzberg:latest \
      serve -H 0.0.0.0 -p 8000
    ```

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
  "version": "4.0.0"
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
  "version": "4.0.0",
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

**CORS Configuration (Critical for Production):**

```bash
# Comma-separated list of allowed origins
KREUZBERG_CORS_ORIGINS="https://app.example.com,https://api.example.com"
```

!!! warning "Security Warning"
    The default CORS configuration allows **all origins** for development convenience. This permits CSRF attacks. **Always set `KREUZBERG_CORS_ORIGINS` in production.**

### Client Examples

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

=== "Python"

    ```python
    import httpx
    from pathlib import Path

    # Single file extraction
    with httpx.Client() as client:
        files = {"files": open("document.pdf", "rb")}
        response = client.post("http://localhost:8000/extract", files=files)
        results = response.json()
        print(results[0]["content"])

    # With configuration
    with httpx.Client() as client:
        files = {"files": open("scanned.pdf", "rb")}
        data = {"config": '{"ocr":{"language":"eng"},"force_ocr":true}'}
        response = client.post(
            "http://localhost:8000/extract",
            files=files,
            data=data
        )
        results = response.json()

    # Multiple files
    with httpx.Client() as client:
        files = [
            ("files", open("doc1.pdf", "rb")),
            ("files", open("doc2.docx", "rb")),
        ]
        response = client.post("http://localhost:8000/extract", files=files)
        results = response.json()
        for result in results:
            print(f"Content: {result['content'][:100]}...")
    ```

=== "TypeScript"

    ```typescript
    // Using fetch API
    const formData = new FormData();
    formData.append("files", fileInput.files[0]);

    const response = await fetch("http://localhost:8000/extract", {
      method: "POST",
      body: formData,
    });

    const results = await response.json();
    console.log(results[0].content);

    // With configuration
    const formDataWithConfig = new FormData();
    formDataWithConfig.append("files", fileInput.files[0]);
    formDataWithConfig.append("config", JSON.stringify({
      ocr: { language: "eng" },
      force_ocr: true
    }));

    const response2 = await fetch("http://localhost:8000/extract", {
      method: "POST",
      body: formDataWithConfig,
    });

    // Multiple files
    const multipleFiles = new FormData();
    for (const file of fileInput.files) {
      multipleFiles.append("files", file);
    }

    const response3 = await fetch("http://localhost:8000/extract", {
      method: "POST",
      body: multipleFiles,
    });
    ```

=== "Ruby"

    ```ruby
    require 'net/http'
    require 'uri'
    require 'json'

    # Single file extraction
    uri = URI('http://localhost:8000/extract')
    request = Net::HTTP::Post.new(uri)
    form_data = [['files', File.open('document.pdf')]]
    request.set_form form_data, 'multipart/form-data'

    response = Net::HTTP.start(uri.hostname, uri.port) do |http|
      http.request(request)
    end

    results = JSON.parse(response.body)
    puts results[0]['content']

    # With configuration
    form_data_with_config = [
      ['files', File.open('scanned.pdf')],
      ['config', '{"ocr":{"language":"eng"},"force_ocr":true}']
    ]
    request.set_form form_data_with_config, 'multipart/form-data'
    ```

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

```python
import httpx

try:
    with httpx.Client() as client:
        files = {"files": open("document.pdf", "rb")}
        response = client.post("http://localhost:8000/extract", files=files)
        response.raise_for_status()
        results = response.json()
except httpx.HTTPStatusError as e:
    error = e.response.json()
    print(f"Error: {error['error_type']}: {error['message']}")
```

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

=== "Python"

    ```python
    import subprocess

    # Start MCP server
    subprocess.Popen(["python", "-m", "kreuzberg", "mcp"])
    ```

=== "Rust"

    ```rust
    use kreuzberg::{ExtractionConfig, mcp::start_mcp_server_with_config};

    #[tokio::main]
    async fn main() -> Result<(), Box<dyn std::error::Error>> {
        let config = ExtractionConfig::discover()?;
        start_mcp_server_with_config(config).await?;
        Ok(())
    }
    ```

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

=== "Custom MCP Client"

    ```python
    import asyncio
    from mcp import ClientSession, StdioServerParameters
    from mcp.client.stdio import stdio_client

    async def main():
        server_params = StdioServerParameters(
            command="kreuzberg",
            args=["mcp"]
        )

        async with stdio_client(server_params) as (read, write):
            async with ClientSession(read, write) as session:
                await session.initialize()

                # List available tools
                tools = await session.list_tools()
                print(f"Available tools: {[t.name for t in tools.tools]}")

                # Call extract_file tool
                result = await session.call_tool(
                    "extract_file",
                    arguments={"path": "document.pdf", "async": True}
                )
                print(result)

    asyncio.run(main())
    ```

=== "LangChain"

    ```python
    from langchain.agents import initialize_agent, AgentType
    from langchain.tools import Tool
    from langchain_openai import ChatOpenAI
    import subprocess
    import json

    # Start MCP server
    mcp_process = subprocess.Popen(
        ["kreuzberg", "mcp"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE
    )

    def extract_file(path: str) -> str:
        request = {
            "method": "tools/call",
            "params": {
                "name": "extract_file",
                "arguments": {"path": path, "async": True}
            }
        }
        mcp_process.stdin.write(json.dumps(request).encode() + b"\n")
        mcp_process.stdin.flush()
        response = mcp_process.stdout.readline()
        return json.loads(response)["result"]["content"]

    tools = [
        Tool(
            name="extract_document",
            func=extract_file,
            description="Extract text from documents (PDF, DOCX, images, etc.)"
        )
    ]

    llm = ChatOpenAI(temperature=0)
    agent = initialize_agent(
        tools, llm, agent=AgentType.ZERO_SHOT_REACT_DESCRIPTION, verbose=True
    )

    agent.run("Extract the content from contract.pdf and summarize it")
    ```

## Production Deployment

### Docker Deployment

**Docker Compose Example:**

```yaml
version: '3.8'

services:
  kreuzberg-api:
    image: goldziher/kreuzberg:v4.0.0-all
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
        image: goldziher/kreuzberg:v4.0.0-all
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

- [ ] **Set `KREUZBERG_CORS_ORIGINS`** to explicit allowed origins
- [ ] **Configure `KREUZBERG_MAX_UPLOAD_SIZE_MB`** based on expected document sizes
- [ ] **Use reverse proxy** (Nginx/Caddy) for SSL/TLS termination
- [ ] **Enable logging** via `RUST_LOG=info` environment variable
- [ ] **Set up health checks** on `/health` endpoint
- [ ] **Monitor cache size** and set up periodic clearing
- [ ] **Use `0.0.0.0` binding** for containerized deployments
- [ ] **Configure resource limits** (CPU, memory) in container orchestration
- [ ] **Test with large files** to validate upload limits and timeouts
- [ ] **Implement rate limiting** at reverse proxy level
- [ ] **Set up monitoring** (Prometheus metrics, logs aggregation)
- [ ] **Plan for horizontal scaling** with load balancing

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
