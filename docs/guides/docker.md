# Docker Deployment

Kreuzberg provides official Docker images built on a high-performance Rust core with Debian 13 (Trixie). Each image supports three execution modes through a flexible entrypoint pattern, enabling deployment as an API server, CLI tool, or MCP server.

## Image Variants

Kreuzberg offers two Docker image variants optimized for different use cases:

### Core Image

**Size:** ~1.0-1.3GB
**Image:** `goldziher/kreuzberg:v4-core`

**Included Features:**
- Tesseract OCR with 12 language packs (eng, spa, fra, deu, ita, por, chi-sim, chi-tra, jpn, ara, rus, hin)
- Pandoc 3.6.3 for document conversion
- pdfium for PDF rendering
- Full support for modern file formats

**Supported Formats:**
- PDF, DOCX, PPTX, XLSX (modern Office formats)
- Images (PNG, JPG, TIFF, BMP, etc.)
- HTML, XML, JSON, YAML, TOML
- Email (EML, MSG)
- Archives (ZIP, TAR, GZ)

**Best For:**
- Production deployments where image size matters
- Cloud environments with size/bandwidth constraints
- Kubernetes deployments with frequent pod scaling
- Workflows that don't require legacy Office format support

### Full Image

**Size:** ~1.5-2.1GB
**Image:** `goldziher/kreuzberg:v4-full`

**Included Features:**
- All Core image features
- LibreOffice 25.8.2 for legacy format conversion

**Additional Formats:**
- Legacy Word (.doc)
- Legacy PowerPoint (.ppt)
- Legacy Excel (.xls)

**Best For:**
- Complete document intelligence pipelines
- Processing legacy MS Office files
- Development and testing environments
- When image size is not a constraint

## Quick Start

### Pull Image

=== "Core"

    ```bash
    docker pull goldziher/kreuzberg:v4-core
    ```

=== "Full"

    ```bash
    docker pull goldziher/kreuzberg:v4-full
    ```

### Basic Usage

=== "API Server"

    ```bash
    # Start API server (default mode)
    docker run -p 8000:8000 goldziher/kreuzberg:v4-core

    # Test the API
    curl -F "files=@document.pdf" http://localhost:8000/extract
    ```

=== "CLI Mode"

    ```bash
    # Extract a single file
    docker run -v $(pwd):/data goldziher/kreuzberg:v4-core \
      extract /data/document.pdf

    # Batch process multiple files
    docker run -v $(pwd):/data goldziher/kreuzberg:v4-core \
      batch /data/*.pdf --output-format json

    # Detect MIME type
    docker run -v $(pwd):/data goldziher/kreuzberg:v4-core \
      detect /data/unknown-file.bin
    ```

=== "MCP Server"

    ```bash
    # Start MCP server
    docker run goldziher/kreuzberg:v4-core mcp
    ```

## Execution Modes

Kreuzberg Docker images use a flexible `ENTRYPOINT` pattern that supports three execution modes:

### 1. API Server Mode (Default)

The default mode starts an HTTP REST API server.

**Default Behavior:**
```bash
docker run -p 8000:8000 goldziher/kreuzberg:v4-core
```

**Custom Configuration:**
```bash
# Change host and port
docker run -p 9000:9000 goldziher/kreuzberg:v4-core \
  serve --host 0.0.0.0 --port 9000

# With environment variables
docker run -p 8000:8000 \
  -e KREUZBERG_CORS_ORIGINS="https://myapp.com" \
  -e KREUZBERG_MAX_UPLOAD_SIZE_MB=200 \
  goldziher/kreuzberg:v4-core

# With configuration file
docker run -p 8000:8000 \
  -v $(pwd)/kreuzberg.toml:/config/kreuzberg.toml \
  goldziher/kreuzberg:v4-core \
  serve --config /config/kreuzberg.toml
```

See [API Server Guide](api-server.md) for complete API documentation.

### 2. CLI Mode

Run Kreuzberg as a command-line tool for file processing.

**Extract Files:**
```bash
# Mount directory and extract file
docker run -v $(pwd):/data goldziher/kreuzberg:v4-core \
  extract /data/document.pdf

# Extract with OCR
docker run -v $(pwd):/data goldziher/kreuzberg:v4-core \
  extract /data/scanned.pdf --ocr true

# Output as JSON
docker run -v $(pwd):/data goldziher/kreuzberg:v4-core \
  extract /data/document.pdf --output-format json > result.json
```

**Batch Processing:**
```bash
# Process multiple files
docker run -v $(pwd):/data goldziher/kreuzberg:v4-core \
  batch /data/*.pdf --output-format json

# With custom concurrency
docker run -v $(pwd):/data goldziher/kreuzberg:v4-core \
  batch /data/*.pdf --concurrency 8
```

**MIME Detection:**
```bash
docker run -v $(pwd):/data goldziher/kreuzberg:v4-core \
  detect /data/unknown-file.bin
```

**Cache Management:**
```bash
# View cache statistics
docker run goldziher/kreuzberg:v4-core cache stats

# Clear cache
docker run goldziher/kreuzberg:v4-core cache clear
```

See [CLI Usage Guide](../cli/usage.md) for complete CLI documentation.

### 3. MCP Server Mode

Run Kreuzberg as a Model Context Protocol server for AI agent integration.

**Start MCP Server:**
```bash
docker run goldziher/kreuzberg:v4-core mcp
```

**With Configuration:**
```bash
docker run \
  -v $(pwd)/kreuzberg.toml:/config/kreuzberg.toml \
  goldziher/kreuzberg:v4-core \
  mcp --config /config/kreuzberg.toml
```

See [API Server Guide - MCP Section](api-server.md#mcp-server) for integration examples.

## Architecture

### Multi-Stage Build

Kreuzberg Docker images use multi-stage builds for optimal size and security:

1. **Builder Stage:** Compiles Rust binary with all dependencies
2. **Runtime Stage:** Minimal Debian Trixie slim base with only runtime dependencies

**Benefits:**
- No build tools or intermediate artifacts in final image
- Smaller image size (builder stage not included)
- Reduced attack surface

### Rust Core

Docker images use the **native Rust core** directly, providing:

- **10-50x performance** over pure-Python alternatives
- **Memory efficiency** through streaming parsers for large files
- **Async processing** with Tokio runtime
- **Zero-copy operations** where possible

### Multi-Architecture Support

Images are built for multiple architectures:

- `linux/amd64` (x86_64)
- `linux/arm64` (aarch64)

Architecture-specific binaries are automatically selected during build.

### Security Features

**Non-Root User:**
```dockerfile
# Images run as unprivileged 'kreuzberg' user
USER kreuzberg
```

**Security Options:**
```bash
# Run with additional security constraints
docker run --security-opt no-new-privileges \
  --read-only \
  --tmpfs /tmp \
  -p 8000:8000 \
  goldziher/kreuzberg:v4-core
```

## Production Deployment

### Docker Compose

**Basic Configuration:**

```yaml
version: '3.8'

services:
  kreuzberg-api:
    image: goldziher/kreuzberg:v4-core
    ports:
      - "8000:8000"
    environment:
      - KREUZBERG_CORS_ORIGINS=https://myapp.com,https://api.myapp.com
      - KREUZBERG_MAX_UPLOAD_SIZE_MB=500
      - RUST_LOG=info
    volumes:
      - ./config:/config
      - cache-data:/app/.kreuzberg
    command: serve --host 0.0.0.0 --port 8000 --config /config/kreuzberg.toml
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "kreuzberg", "--version"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 5s

volumes:
  cache-data:
```

**With LibreOffice (Full Image):**

```yaml
services:
  kreuzberg-full:
    image: goldziher/kreuzberg:v4-full
    ports:
      - "8000:8000"
    environment:
      - KREUZBERG_CORS_ORIGINS=https://myapp.com
    volumes:
      - cache-data:/app/.kreuzberg
    restart: unless-stopped
```

**Start Services:**
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
  labels:
    app: kreuzberg
spec:
  replicas: 3
  selector:
    matchLabels:
      app: kreuzberg
  template:
    metadata:
      labels:
        app: kreuzberg
    spec:
      containers:
      - name: kreuzberg
        image: goldziher/kreuzberg:v4-core
        ports:
        - containerPort: 8000
          name: http
        env:
        - name: KREUZBERG_CORS_ORIGINS
          value: "https://myapp.com"
        - name: KREUZBERG_MAX_UPLOAD_SIZE_MB
          value: "500"
        - name: RUST_LOG
          value: "info"
        args: ["serve", "--host", "0.0.0.0", "--port", "8000"]
        livenessProbe:
          exec:
            command:
            - kreuzberg
            - --version
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
        volumeMounts:
        - name: cache
          mountPath: /app/.kreuzberg
      volumes:
      - name: cache
        emptyDir: {}
---
apiVersion: v1
kind: Service
metadata:
  name: kreuzberg-api
spec:
  selector:
    app: kreuzberg
  ports:
  - protocol: TCP
    port: 80
    targetPort: 8000
  type: LoadBalancer
```

**Apply Configuration:**
```bash
kubectl apply -f kreuzberg-deployment.yaml
```

### Environment Variables

Configure Docker containers via environment variables:

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

**Logging:**
```bash
RUST_LOG=info                    # Logging level (error, warn, info, debug, trace)
```

**Cache Configuration:**
```bash
KREUZBERG_CACHE_DIR=/app/.kreuzberg  # Cache directory (default)
```

### Volume Mounts

**Cache Persistence:**
```bash
# Mount cache directory for persistence
docker run -p 8000:8000 \
  -v kreuzberg-cache:/app/.kreuzberg \
  goldziher/kreuzberg:v4-core
```

**Configuration Files:**
```bash
# Mount configuration file
docker run -p 8000:8000 \
  -v $(pwd)/kreuzberg.toml:/config/kreuzberg.toml \
  goldziher/kreuzberg:v4-core \
  serve --config /config/kreuzberg.toml
```

**File Processing:**
```bash
# Mount documents directory (read-only)
docker run -v $(pwd)/documents:/data:ro \
  goldziher/kreuzberg:v4-core \
  extract /data/document.pdf
```

## Image Comparison

| Feature | Core | Full | Difference |
|---------|------|------|------------|
| **Base Image** | debian:trixie-slim | debian:trixie-slim | - |
| **Size** | ~1.0-1.3GB | ~1.5-2.1GB | ~500-800MB |
| **Tesseract OCR** | ✅ 12 languages | ✅ 12 languages | - |
| **Pandoc** | ✅ 3.6.3 | ✅ 3.6.3 | - |
| **pdfium** | ✅ | ✅ | - |
| **Modern Office** | ✅ DOCX, PPTX, XLSX | ✅ DOCX, PPTX, XLSX | - |
| **Legacy Office** | ❌ | ✅ DOC, PPT, XLS | LibreOffice 25.8.2 |
| **Pull Time** | ~30s | ~45s | ~15s slower |
| **Startup Time** | ~1s | ~1s | Negligible |

## Building Custom Images

### Building from Source

Clone the repository and build:

=== "Core Image"

    ```bash
    git clone https://github.com/Goldziher/kreuzberg.git
    cd kreuzberg
    docker build -f docker/Dockerfile.core -t kreuzberg:core .
    ```

=== "Full Image"

    ```bash
    git clone https://github.com/Goldziher/kreuzberg.git
    cd kreuzberg
    docker build -f docker/Dockerfile.full -t kreuzberg:full .
    ```

### Custom Dockerfiles

Create a custom Dockerfile based on official images:

```dockerfile
FROM goldziher/kreuzberg:v4-core

# Install additional system dependencies
USER root
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        your-package-here && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Switch back to non-root user
USER kreuzberg

# Custom configuration
COPY kreuzberg.toml /app/kreuzberg.toml

# Custom entrypoint
CMD ["serve", "--config", "/app/kreuzberg.toml"]
```

## Performance Tuning

### Resource Allocation

**Recommended Resources:**

| Workload | Memory | CPU | Notes |
|----------|--------|-----|-------|
| Light | 512MB | 0.5 cores | Small documents, low concurrency |
| Medium | 1GB | 1 core | Typical documents, moderate concurrency |
| Heavy | 2GB+ | 2+ cores | Large documents, OCR, high concurrency |

**Docker Run:**
```bash
docker run -p 8000:8000 \
  --memory=1g \
  --cpus=1 \
  goldziher/kreuzberg:v4-core
```

**Docker Compose:**
```yaml
services:
  kreuzberg:
    image: goldziher/kreuzberg:v4-core
    deploy:
      resources:
        limits:
          memory: 1G
          cpus: '1'
        reservations:
          memory: 512M
          cpus: '0.5'
```

### Scaling

**Horizontal Scaling:**
```bash
# Scale to 5 replicas
docker-compose up -d --scale kreuzberg-api=5

# Kubernetes
kubectl scale deployment kreuzberg-api --replicas=5
```

**Load Balancing:**
- Use reverse proxy (Nginx, Caddy, Traefik)
- Kubernetes Service with LoadBalancer type
- Docker Swarm mode

## Troubleshooting

### Container Won't Start

**Check logs:**
```bash
docker logs <container-id>
```

**Common Issues:**
- Port already in use: Change `-p` mapping
- Insufficient permissions: Ensure volume mounts have correct permissions
- Memory limit too low: Increase `--memory` limit

### Permission Errors

Images run as non-root user `kreuzberg` (UID 1000). Ensure mounted volumes have correct permissions:

```bash
# Fix permissions on mounted directory
chown -R 1000:1000 /path/to/mounted/directory
```

### Large File Processing

**Increase memory limit:**
```bash
docker run -p 8000:8000 \
  --memory=4g \
  goldziher/kreuzberg:v4-core
```

**Increase upload size:**
```bash
docker run -p 8000:8000 \
  -e KREUZBERG_MAX_UPLOAD_SIZE_MB=1000 \
  goldziher/kreuzberg:v4-core
```

### LibreOffice Not Available

LibreOffice is only available in the **Full** image variant. If you need legacy Office format support:

```bash
# Switch to full image
docker pull goldziher/kreuzberg:v4-full
docker run -p 8000:8000 goldziher/kreuzberg:v4-full
```

## Next Steps

- [API Server Guide](api-server.md) - Complete API documentation
- [CLI Usage](../cli/usage.md) - Command-line interface
- [Configuration](configuration.md) - Configuration options
- [Advanced Features](advanced.md) - Chunking, language detection, token reduction
