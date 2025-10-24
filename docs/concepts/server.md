# Server Features

Kreuzberg provides production-ready HTTP servers for document extraction: a RESTful API server and an MCP (Model Context Protocol) server.

## API Server

The API server provides RESTful HTTP endpoints for document extraction with multipart file uploads, batch processing, and cache management.

### Architecture

- **Framework**: Axum (high-performance async Rust web framework)
- **CORS**: Enabled for all origins (configurable)
- **Tracing**: HTTP request/response logging
- **Multipart Upload**: Support for single and multiple file uploads
- **Config Override**: Per-request configuration overrides server defaults
- **Config Discovery**: Automatic discovery of `kreuzberg.toml/yaml/json` files

### Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/extract` | POST | Extract text from uploaded files (multipart form data) |
| `/health` | GET | Health check endpoint (returns status and version) |
| `/info` | GET | Server information (version, backend type) |
| `/cache/stats` | GET | Cache statistics (size, file count, age) |
| `/cache/clear` | DELETE | Clear cache directory |

## Quick Start

### Starting the Server

=== "Rust"

    ```rust
    use kreuzberg::api::serve;

    #[tokio::main]
    async fn main() -> kreuzberg::Result<()> {
        // Local development (127.0.0.1:8000)
        serve("127.0.0.1", 8000).await?;
        Ok(())
    }
    ```

    ```rust
    use kreuzberg::api::serve;

    #[tokio::main]
    async fn main() -> kreuzberg::Result<()> {
        // Docker/production (listen on all interfaces)
        serve("0.0.0.0", 8000).await?;
        Ok(())
    }
    ```

    ```bash
    # Build and run
    cargo build --release --features api
    cargo run --release --features api --bin kreuzberg-server
    ```

=== "Python"

    ```bash
    # Python uses the embedded CLI binary to start the server
    python -m kreuzberg serve --host 127.0.0.1 --port 8000

    # With custom config
    python -m kreuzberg serve --host 0.0.0.0 --port 8000 --config kreuzberg.toml
    ```

=== "TypeScript"

    ```bash
    # TypeScript uses the embedded CLI binary to start the server
    npx kreuzberg serve --host 127.0.0.1 --port 8000

    # With custom config
    npx kreuzberg serve --host 0.0.0.0 --port 8000 --config kreuzberg.toml
    ```

=== "CLI (Direct)"

    ```bash
    # Install and use the Rust CLI directly
    cargo install kreuzberg-cli --features all
    kreuzberg serve --host 0.0.0.0 --port 8000

    # With custom config
    kreuzberg serve --host 0.0.0.0 --port 8000 --config kreuzberg.toml
    ```

=== "Docker"

    ```bash
    # Run the API server in Docker
    docker run -d -p 8000:8000 goldziher/kreuzberg:v4.0.0

    # With custom config volume
    docker run -d -p 8000:8000 \
      -v $(pwd)/kreuzberg.toml:/app/kreuzberg.toml \
      goldziher/kreuzberg:v4.0.0

    # With cache persistence
    docker run -d -p 8000:8000 \
      -v $(pwd)/.kreuzberg:/app/.kreuzberg \
      goldziher/kreuzberg:v4.0.0
    ```

### Using the API

#### Single File Extraction

=== "cURL"

    ```bash
    # Extract a single PDF
    curl -F "files=@document.pdf" http://localhost:8000/extract

    # With custom MIME type
    curl -F "files=@document.pdf;type=application/pdf" \
         http://localhost:8000/extract

    # Save output to file
    curl -F "files=@document.pdf" \
         http://localhost:8000/extract \
         -o result.json
    ```

=== "Python (requests)"

    ```python
    import requests

    url = "http://localhost:8000/extract"

    # Single file
    with open("document.pdf", "rb") as f:
        files = {"files": f}
        response = requests.post(url, files=files)

    results = response.json()
    print(results[0]["content"])
    ```

=== "TypeScript (fetch)"

    ```typescript
    // Single file extraction
    const formData = new FormData();
    const file = await fetch('document.pdf').then(r => r.blob());
    formData.append('files', file, 'document.pdf');

    const response = await fetch('http://localhost:8000/extract', {
      method: 'POST',
      body: formData
    });

    const results = await response.json();
    console.log(results[0].content);
    ```

=== "TypeScript (axios)"

    ```typescript
    import axios from 'axios';
    import FormData from 'form-data';
    import fs from 'fs';

    const formData = new FormData();
    formData.append('files', fs.createReadStream('document.pdf'));

    const response = await axios.post('http://localhost:8000/extract', formData, {
      headers: formData.getHeaders()
    });

    const results = response.data;
    console.log(results[0].content);
    ```

#### Batch Extraction

=== "cURL"

    ```bash
    # Multiple files in one request
    curl -F "files=@doc1.pdf" \
         -F "files=@doc2.pdf" \
         -F "files=@image.jpg" \
         http://localhost:8000/extract
    ```

=== "Python (requests)"

    ```python
    import requests

    url = "http://localhost:8000/extract"

    # Multiple files
    files = [
        ("files", ("doc1.pdf", open("doc1.pdf", "rb"), "application/pdf")),
        ("files", ("doc2.pdf", open("doc2.pdf", "rb"), "application/pdf")),
        ("files", ("image.jpg", open("image.jpg", "rb"), "image/jpeg")),
    ]

    response = requests.post(url, files=files)
    results = response.json()

    for i, result in enumerate(results):
        print(f"Document {i + 1}: {len(result['content'])} characters")
    ```

=== "TypeScript (fetch)"

    ```typescript
    const formData = new FormData();

    const files = ['doc1.pdf', 'doc2.pdf', 'image.jpg'];
    for (const filename of files) {
      const blob = await fetch(filename).then(r => r.blob());
      formData.append('files', blob, filename);
    }

    const response = await fetch('http://localhost:8000/extract', {
      method: 'POST',
      body: formData
    });

    const results = await response.json();
    results.forEach((result, i) => {
      console.log(`Document ${i + 1}: ${result.content.length} characters`);
    });
    ```

#### Custom Configuration

=== "cURL"

    ```bash
    # With OCR configuration
    curl -F "files=@scanned.pdf" \
         -F 'config={"ocr":{"language":"eng","backend":"tesseract"}}' \
         http://localhost:8000/extract

    # With chunking and language detection
    curl -F "files=@document.pdf" \
         -F 'config={"chunking":{"max_chunk_size":1000,"overlap":100},"language_detection":{"enabled":true}}' \
         http://localhost:8000/extract

    # Disable quality processing
    curl -F "files=@document.pdf" \
         -F 'config={"enable_quality_processing":false}' \
         http://localhost:8000/extract
    ```

=== "Python (requests)"

    ```python
    import requests
    import json

    url = "http://localhost:8000/extract"

    # Custom config with OCR
    config = {
        "ocr": {
            "language": "eng+deu",
            "backend": "tesseract"
        },
        "chunking": {
            "max_chunk_size": 1000,
            "overlap": 100
        },
        "enable_quality_processing": True
    }

    files = {
        "files": open("scanned.pdf", "rb"),
        "config": (None, json.dumps(config), "application/json")
    }

    response = requests.post(url, files=files)
    results = response.json()
    ```

=== "TypeScript (fetch)"

    ```typescript
    const formData = new FormData();
    const file = await fetch('scanned.pdf').then(r => r.blob());
    formData.append('files', file, 'scanned.pdf');

    // Custom config
    const config = {
      ocr: {
        language: 'eng+deu',
        backend: 'tesseract'
      },
      chunking: {
        maxChunkSize: 1000,
        overlap: 100
      },
      enableQualityProcessing: true
    };

    formData.append('config', JSON.stringify(config));

    const response = await fetch('http://localhost:8000/extract', {
      method: 'POST',
      body: formData
    });

    const results = await response.json();
    ```

### Health Check

=== "cURL"

    ```bash
    # Health check
    curl http://localhost:8000/health
    ```

    **Response:**
    ```json
    {
      "status": "healthy",
      "version": "4.0.0"
    }
    ```

=== "Python (requests)"

    ```python
    import requests

    response = requests.get("http://localhost:8000/health")
    print(response.json())
    # {"status": "healthy", "version": "4.0.0"}
    ```

=== "TypeScript (fetch)"

    ```typescript
    const response = await fetch('http://localhost:8000/health');
    const health = await response.json();
    console.log(health);
    // {"status": "healthy", "version": "4.0.0"}
    ```

### Server Info

=== "cURL"

    ```bash
    # Server information
    curl http://localhost:8000/info
    ```

    **Response:**
    ```json
    {
      "version": "4.0.0",
      "rust_backend": true
    }
    ```

=== "Python (requests)"

    ```python
    import requests

    response = requests.get("http://localhost:8000/info")
    print(response.json())
    ```

=== "TypeScript (fetch)"

    ```typescript
    const response = await fetch('http://localhost:8000/info');
    const info = await response.json();
    console.log(info);
    ```

### Cache Management

#### Cache Statistics

=== "cURL"

    ```bash
    # Get cache statistics
    curl http://localhost:8000/cache/stats
    ```

    **Response:**
    ```json
    {
      "directory": "/app/.kreuzberg",
      "total_files": 42,
      "total_size_mb": 156.8,
      "available_space_mb": 15420.5,
      "oldest_file_age_days": 14.2,
      "newest_file_age_days": 0.1
    }
    ```

=== "Python (requests)"

    ```python
    import requests

    response = requests.get("http://localhost:8000/cache/stats")
    stats = response.json()
    print(f"Cache size: {stats['total_size_mb']:.1f} MB")
    print(f"Cache files: {stats['total_files']}")
    ```

=== "TypeScript (fetch)"

    ```typescript
    const response = await fetch('http://localhost:8000/cache/stats');
    const stats = await response.json();
    console.log(`Cache size: ${stats.total_size_mb.toFixed(1)} MB`);
    console.log(`Cache files: ${stats.total_files}`);
    ```

#### Clear Cache

=== "cURL"

    ```bash
    # Clear all cached files
    curl -X DELETE http://localhost:8000/cache/clear
    ```

    **Response:**
    ```json
    {
      "directory": "/app/.kreuzberg",
      "removed_files": 42,
      "freed_mb": 156.8
    }
    ```

=== "Python (requests)"

    ```python
    import requests

    response = requests.delete("http://localhost:8000/cache/clear")
    result = response.json()
    print(f"Cleared {result['removed_files']} files")
    print(f"Freed {result['freed_mb']:.1f} MB")
    ```

=== "TypeScript (fetch)"

    ```typescript
    const response = await fetch('http://localhost:8000/cache/clear', {
      method: 'DELETE'
    });
    const result = await response.json();
    console.log(`Cleared ${result.removed_files} files`);
    console.log(`Freed ${result.freed_mb.toFixed(1)} MB`);
    ```

## Advanced Configuration

### Embedding the Router

You can embed the Kreuzberg router in your own Axum application:

=== "Rust"

    ```rust
    use kreuzberg::{ExtractionConfig, api::create_router};
    use axum::{Router, routing::get};

    #[tokio::main]
    async fn main() -> kreuzberg::Result<()> {
        // Load config
        let config = ExtractionConfig::default();

        // Create Kreuzberg router
        let kreuzberg_router = create_router(config);

        // Create your own routes
        let app = Router::new()
            .route("/", get(|| async { "Hello, World!" }))
            .nest("/api/docs", kreuzberg_router);  // Nest under /api/docs

        // Serve
        let listener = tokio::net::TcpListener::bind("127.0.0.1:8000")
            .await
            .unwrap();

        axum::serve(listener, app).await.unwrap();

        Ok(())
    }
    ```

### Custom Configuration Loading

=== "Rust"

    ```rust
    use kreuzberg::{ExtractionConfig, api::serve_with_config};

    #[tokio::main]
    async fn main() -> kreuzberg::Result<()> {
        // Load from specific file
        let config = ExtractionConfig::from_toml_file("config/production.toml")?;

        // Start server with this config
        serve_with_config("0.0.0.0", 8000, config).await?;

        Ok(())
    }
    ```

### Configuration File Discovery

The server automatically searches for configuration files in this order:

1. `./kreuzberg.toml`
2. `./kreuzberg.yaml`
3. `./kreuzberg.json`
4. `../kreuzberg.toml`
5. `../kreuzberg.yaml`
6. `../kreuzberg.json`

If no file is found, uses default configuration.

**Example `kreuzberg.toml`:**

```toml
# Enable OCR by default
[ocr]
backend = "tesseract"
language = "eng"

# Enable quality processing
enable_quality_processing = true

# Enable caching
use_cache = true

# Chunking configuration
[chunking]
max_chunk_size = 1000
overlap = 100
```

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `KREUZBERG_HOST` | Server bind address | `127.0.0.1` |
| `KREUZBERG_PORT` | Server port | `8000` |
| `RUST_LOG` | Log level (error, warn, info, debug, trace) | `info` |

=== "Bash"

    ```bash
    # Set environment variables
    export KREUZBERG_HOST=0.0.0.0
    export KREUZBERG_PORT=3000
    export RUST_LOG=debug

    # Start server
    cargo run --release --features api --bin kreuzberg-server
    ```

=== "Docker"

    ```bash
    # Pass environment variables to Docker
    docker run -d \
      -e KREUZBERG_HOST=0.0.0.0 \
      -e KREUZBERG_PORT=8000 \
      -e RUST_LOG=info \
      -p 8000:8000 \
      goldziher/kreuzberg:v4.0.0
    ```

## Error Handling

All errors return JSON with this structure:

```json
{
  "error_type": "ValidationError",
  "message": "No files provided for extraction",
  "status_code": 400
}
```

### HTTP Status Codes

| Code | Type | Description |
|------|------|-------------|
| 200 | Success | Request completed successfully |
| 400 | Validation Error | Invalid request (missing files, bad config) |
| 422 | Parsing Error | Document parsing failed |
| 500 | Internal Error | Unexpected server error |
| 503 | Dependency Error | Missing dependency (OCR engine, etc.) |

### Error Types

- `ValidationError`: Invalid input or configuration
- `ParsingError`: Document format not supported or corrupted
- `OcrError`: OCR processing failed
- `MissingDependencyError`: Required external tool not installed
- `InternalError`: Unexpected error

=== "Python (with error handling)"

    ```python
    import requests

    try:
        response = requests.post(
            "http://localhost:8000/extract",
            files={"files": open("document.pdf", "rb")}
        )
        response.raise_for_status()
        results = response.json()
    except requests.HTTPError as e:
        error = e.response.json()
        print(f"Error ({error['status_code']}): {error['message']}")
    ```

=== "TypeScript (with error handling)"

    ```typescript
    try {
      const response = await fetch('http://localhost:8000/extract', {
        method: 'POST',
        body: formData
      });

      if (!response.ok) {
        const error = await response.json();
        throw new Error(`Error (${error.status_code}): ${error.message}`);
      }

      const results = await response.json();
    } catch (error) {
      console.error('Extraction failed:', error);
    }
    ```

## Performance and Scaling

### Performance Tips

1. **Use Batch Extraction**: Process multiple files in one request for better throughput
2. **Enable Caching**: Avoid re-processing identical files
3. **Adjust Config Per Request**: Override heavy features (OCR, chunking) only when needed
4. **Connection Pooling**: Reuse HTTP connections for multiple requests

### Horizontal Scaling

The API server is stateless (except for local cache) and can be scaled horizontally:

=== "Docker Compose"

    ```yaml
    version: '3.8'
    services:
      kreuzberg:
        image: goldziher/kreuzberg:v4.0.0
        ports:
          - "8000:8000"
        environment:
          - KREUZBERG_HOST=0.0.0.0
          - KREUZBERG_PORT=8000
        volumes:
          - ./kreuzberg.toml:/app/kreuzberg.toml
          - cache:/app/.kreuzberg
        deploy:
          replicas: 3

      nginx:
        image: nginx:latest
        ports:
          - "80:80"
        volumes:
          - ./nginx.conf:/etc/nginx/nginx.conf
        depends_on:
          - kreuzberg

    volumes:
      cache:
    ```

=== "Kubernetes"

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
            image: goldziher/kreuzberg:v4.0.0
            ports:
            - containerPort: 8000
            env:
            - name: KREUZBERG_HOST
              value: "0.0.0.0"
            - name: KREUZBERG_PORT
              value: "8000"
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
      name: kreuzberg-service
    spec:
      selector:
        app: kreuzberg-api
      ports:
      - protocol: TCP
        port: 80
        targetPort: 8000
      type: LoadBalancer
    ```

### Load Testing

=== "Bash (using ab)"

    ```bash
    # Apache Bench - 1000 requests, 10 concurrent
    ab -n 1000 -c 10 -p document.pdf -T multipart/form-data \
       http://localhost:8000/extract
    ```

=== "Python (using locust)"

    ```python
    from locust import HttpUser, task, between

    class KreuzbergUser(HttpUser):
        wait_time = between(1, 3)

        @task
        def extract_document(self):
            with open("test.pdf", "rb") as f:
                self.client.post("/extract", files={"files": f})
    ```

## MCP Server

The MCP (Model Context Protocol) server provides a standardized interface for AI agents and language models to interact with Kreuzberg's document extraction capabilities.

### What is MCP?

The [Model Context Protocol](https://modelcontextprotocol.io/) is an open protocol that standardizes how AI systems access context from external sources. Kreuzberg's MCP server exposes document extraction as "tools" that AI agents can invoke.

### Features

- **Standardized Interface**: Compatible with any MCP client (Claude Desktop, custom agents)
- **Tool-Based API**: AI agents can call extraction functions as tools
- **Streaming Support**: Progress updates during extraction
- **Configuration**: Per-request config overrides

### MCP Tools

| Tool | Description | Arguments |
|------|-------------|-----------|
| `extract_file` | Extract text from a file path | `file_path`, `config` (optional) |
| `extract_bytes` | Extract text from base64-encoded bytes | `data`, `mime_type`, `config` (optional) |
| `batch_extract` | Extract from multiple files | `file_paths`, `config` (optional) |

### Starting the MCP Server

=== "Rust"

    ```rust
    use kreuzberg::mcp::serve;

    #[tokio::main]
    async fn main() -> kreuzberg::Result<()> {
        // MCP uses stdio transport
        serve().await?;
        Ok(())
    }
    ```

    ```bash
    # Build and run
    cargo build --release --features mcp
    cargo run --release --features mcp --bin kreuzberg-mcp
    ```

=== "Python"

    ```bash
    # Python uses the embedded CLI binary to start the MCP server
    python -m kreuzberg mcp

    # With custom config
    python -m kreuzberg mcp --config kreuzberg.toml
    ```

=== "TypeScript"

    ```bash
    # TypeScript uses the embedded CLI binary to start the MCP server
    npx kreuzberg mcp

    # With custom config
    npx kreuzberg mcp --config kreuzberg.toml
    ```

=== "CLI (Direct)"

    ```bash
    # Install and use the Rust CLI directly
    cargo install kreuzberg-cli --features all

    # Start MCP server
    kreuzberg mcp

    # With custom config
    kreuzberg mcp --config kreuzberg.toml
    ```

### Claude Desktop Integration

Add Kreuzberg MCP server to Claude Desktop configuration:

=== "macOS"

    **File**: `~/Library/Application Support/Claude/claude_desktop_config.json`

    ```json
    {
      "mcpServers": {
        "kreuzberg": {
          "command": "/path/to/kreuzberg-mcp",
          "args": []
        }
      }
    }
    ```

=== "Windows"

    **File**: `%APPDATA%\Claude\claude_desktop_config.json`

    ```json
    {
      "mcpServers": {
        "kreuzberg": {
          "command": "C:\\path\\to\\kreuzberg-mcp.exe",
          "args": []
        }
      }
    }
    ```

=== "Linux"

    **File**: `~/.config/Claude/claude_desktop_config.json`

    ```json
    {
      "mcpServers": {
        "kreuzberg": {
          "command": "/path/to/kreuzberg-mcp",
          "args": []
        }
      }
    }
    ```

### Using MCP Tools

Once configured, Claude can use Kreuzberg tools:

**User**: "Extract text from my document.pdf file"

**Claude**: *Uses `extract_file` tool*
```json
{
  "tool": "extract_file",
  "arguments": {
    "file_path": "/Users/user/Documents/document.pdf"
  }
}
```

**Response**:
```json
{
  "content": "Document text here...",
  "metadata": {...},
  "tables": [...]
}
```

### MCP Configuration

You can provide a default config file for the MCP server:

```bash
# Place kreuzberg.toml in the same directory as the MCP binary
./kreuzberg-mcp

# Or specify via environment variable
KREUZBERG_CONFIG=/path/to/config.toml ./kreuzberg-mcp
```

## Feature Flags

Server features require specific Cargo features:

```toml
[dependencies]
kreuzberg = { version = "4.0", features = ["api", "mcp"] }
```

| Feature | Description | Dependencies |
|---------|-------------|--------------|
| `api` | REST API server | `axum`, `tower`, `tower-http`, `tracing` |
| `mcp` | MCP server | `rmcp` |
| `all` | Both API and MCP | `api`, `mcp` |

**Note**: Python and TypeScript packages include the CLI binary with all server features. The `serve` and `mcp` commands are available via `python -m kreuzberg` and `npx kreuzberg` respectively.

## Docker Images

Official Docker images include API server pre-configured:

| Image | Description | Size |
|-------|-------------|------|
| `goldziher/kreuzberg:v4.0.0` | Core + Tesseract OCR + API | ~500MB |
| `goldziher/kreuzberg:v4.0.0-easyocr` | Core + EasyOCR + API | ~2GB |
| `goldziher/kreuzberg:v4.0.0-paddle` | Core + PaddleOCR + API | ~1.5GB |
| `goldziher/kreuzberg:v4.0.0-all` | All features + API | ~3GB |

### Docker Compose Example

```yaml
version: '3.8'
services:
  kreuzberg-api:
    image: goldziher/kreuzberg:v4.0.0
    ports:
      - "8000:8000"
    environment:
      - KREUZBERG_HOST=0.0.0.0
      - KREUZBERG_PORT=8000
      - RUST_LOG=info
    volumes:
      - ./kreuzberg.toml:/app/kreuzberg.toml:ro
      - cache-volume:/app/.kreuzberg
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8000/health"]
      interval: 30s
      timeout: 10s
      retries: 3

volumes:
  cache-volume:
```

## Security Considerations

### File Upload Limits

Configure maximum file upload sizes:

=== "Rust (Axum)"

    ```rust
    use axum::Router;
    use tower_http::limit::RequestBodyLimitLayer;

    let app = create_router(config)
        .layer(RequestBodyLimitLayer::new(50 * 1024 * 1024)); // 50MB limit
    ```

### CORS Configuration

The default configuration allows all origins. For production, restrict CORS:

=== "Rust"

    ```rust
    use tower_http::cors::{CorsLayer, AllowOrigin};

    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list(["https://yourdomain.com".parse().unwrap()]))
        .allow_methods(Any)
        .allow_headers(Any);

    let app = create_router(config).layer(cors);
    ```

### Authentication

The server does not include authentication. Use a reverse proxy (nginx, Traefik) for:

- API key authentication
- JWT validation
- Rate limiting
- TLS/SSL termination

## Monitoring and Observability

### Structured Logging

Enable detailed logging with `RUST_LOG`:

```bash
export RUST_LOG=kreuzberg=debug,axum=info,tower_http=debug
```

Log levels:
- `error`: Errors only
- `warn`: Warnings and errors
- `info`: Informational messages (default)
- `debug`: Detailed debugging info
- `trace`: Very detailed tracing

### Metrics

The server includes HTTP tracing via `tower-http`. Integrate with:

- **Prometheus**: Use `axum-prometheus` crate
- **OpenTelemetry**: Use `tracing-opentelemetry` crate
- **Datadog**: Use `datadog-tracing` crate

### Health Checks

Use `/health` endpoint for:

- Kubernetes liveness/readiness probes
- Load balancer health checks
- Monitoring dashboards

## Troubleshooting

### Server Won't Start

**Error**: `Address already in use`

**Solution**: Port 8000 is occupied. Use a different port or kill the existing process:

```bash
# Find process using port 8000
lsof -i :8000

# Kill the process
kill -9 <PID>

# Or use a different port
serve("127.0.0.1", 3000).await?;
```

### Large File Uploads Fail

**Error**: `Request body too large`

**Solution**: Increase body size limit (see Security Considerations above)

### OCR Not Working

**Error**: `MissingDependencyError: tesseract not found`

**Solution**: Install Tesseract:

```bash
# macOS
brew install tesseract

# Ubuntu/Debian
sudo apt-get install tesseract-ocr

# Docker: Use kreuzberg image (includes Tesseract)
docker run -p 8000:8000 goldziher/kreuzberg:v4.0.0
```

### Cache Issues

**Error**: `Permission denied` when accessing cache

**Solution**: Ensure cache directory is writable:

```bash
chmod 755 .kreuzberg
```

## See Also

- [Architecture](architecture.md) - System design
- [Extractors](extractors.md) - Supported formats
- [Text Processing](text-processing.md) - Quality, keywords, token reduction
- [Plugin System](plugins.md) - Extending Kreuzberg
