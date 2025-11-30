# Docker

Kreuzberg provides optimized Docker images for easy deployment and containerized usage.

## Available Images

Docker images are available on [Docker Hub](https://hub.docker.com/r/goldziher/kreuzberg).

### Base Image (`latest`)

- **Image**: `goldziher/kreuzberg:latest`
- **Size**: ~550MB compressed
- **Includes**:
    - ✅ REST API server
    - ✅ CLI tools
    - ✅ Tesseract OCR with 12 business languages
    - ✅ Core document extraction for all supported formats

**Use Cases:**

- Basic document processing workflows
- Simple REST API deployments
- CLI-based batch processing
- Microservices requiring text extraction
- Cost-conscious deployments where advanced features aren't needed

**Languages Supported:** English, Spanish, French, German, Italian, Portuguese, Chinese (Simplified & Traditional), Japanese, Arabic, Russian, Hindi

### Core Image (`core`)

- **Image**: `goldziher/kreuzberg-core:latest`
- **Size**: ~700MB compressed
- **Includes**: Everything from base plus:
    - ✅ Text chunking (semantic-text-splitter)
    - ✅ Encrypted PDF support (crypto)
    - ✅ Document classification
    - ✅ Language detection
    - ✅ Email parsing (.eml, .msg files)
    - ✅ Additional format extensions

**Use Cases:**

- RAG (Retrieval-Augmented Generation) applications
- Document intelligence workflows
- Multi-language document processing
- Enterprise document management systems
- Advanced content analysis pipelines

## Quick Start

### Basic Usage

```bash
# Pull and run the base image
docker pull goldziher/kreuzberg:latest
docker run -p 8000:8000 goldziher/kreuzberg:latest

# Extract text from a document
curl -X POST http://localhost:8000/extract \
  -F "data=@document.pdf"
```

### Core Image with Advanced Features

```bash
# Run core image with all features
docker run -p 8000:8000 goldziher/kreuzberg-core:latest

# Extract with chunking for RAG
curl -X POST http://localhost:8000/extract \
  -F "data=@document.pdf" \
  -F "chunk_content=true" \
  -F "max_chars=1000"

# Extract with language detection
curl -X POST http://localhost:8000/extract \
  -F "data=@document.pdf" \
  -F "auto_detect_language=true"

# Extract from encrypted PDF
curl -X POST http://localhost:8000/extract \
  -F "data=@encrypted.pdf" \
  -F "password=secretpassword"
```

### With Persistent Cache

```bash
# Create cache directory
mkdir -p kreuzberg-cache

# Run with persistent cache (recommended for production)
docker run -p 8000:8000 \
  -v "$(pwd)/kreuzberg-cache:/app/.kreuzberg" \
  goldziher/kreuzberg-core:latest
```

## Custom Docker Images

For production deployments, you can create custom images tailored to your specific needs.

### Example: Lightweight Custom Image

This example adds only language detection to the base image:

```dockerfile
FROM goldziher/kreuzberg:latest

USER root

# Add language detection capability
RUN pip install --upgrade "kreuzberg[langdetect]"

USER appuser

# Set default configuration
ENV KREUZBERG_AUTO_DETECT_LANGUAGE=true
```

Build and test:

```bash
# Build the custom image
docker build -t kreuzberg-custom .

# Test the functionality
docker run -d --name test-kreuzberg -p 8000:8000 kreuzberg-custom

# Verify language detection works
curl -X POST http://localhost:8000/extract \
  -F "data=@sample.pdf" \
  -F "auto_detect_language=true"

# Cleanup
docker stop test-kreuzberg && docker rm test-kreuzberg
```

### Example: RAG-Optimized Image

This example creates an image optimized for RAG applications:

```dockerfile
FROM goldziher/kreuzberg:latest

USER root

# Install chunking and language detection
RUN pip install --upgrade "kreuzberg[chunking,langdetect]"

USER appuser

# RAG-optimized defaults
ENV KREUZBERG_CHUNK_CONTENT=true
ENV KREUZBERG_MAX_CHARS=1000
ENV KREUZBERG_MAX_OVERLAP=100
ENV KREUZBERG_AUTO_DETECT_LANGUAGE=true
```

### Example: Multi-Format Processing Image

This example adds crypto support for encrypted PDFs and email parsing:

```dockerfile
FROM goldziher/kreuzberg:latest

USER root

# Add crypto and email support
RUN pip install --upgrade "kreuzberg[crypto]"

USER appuser

# Enable crypto support
ENV KREUZBERG_CRYPTO_SUPPORT=true
```

## Docker Compose

### Production Setup

```yaml
version: '3.8'

services:
  kreuzberg:
    image: goldziher/kreuzberg-core:latest
    ports:
      - "8000:8000"
    volumes:
      - kreuzberg-cache:/app/.kreuzberg
      - ./config/kreuzberg.toml:/app/kreuzberg.toml:ro
    environment:
      - PYTHONUNBUFFERED=1
      - KREUZBERG_CACHE_DIR=/app/.kreuzberg
      # Cache limits (adjust based on your needs)
      - KREUZBERG_OCR_CACHE_SIZE_MB=500
      - KREUZBERG_DOCUMENT_CACHE_SIZE_MB=1000
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8000/health"]
      interval: 30s
      timeout: 10s
      retries: 3
    deploy:
      resources:
        limits:
          memory: 2G
          cpus: '2.0'
        reservations:
          memory: 1G
          cpus: '1.0'

volumes:
  kreuzberg-cache:
```

### Development Setup

```yaml
version: '3.8'

services:
  kreuzberg-dev:
    image: goldziher/kreuzberg-core:latest
    ports:
      - "8000:8000"
    volumes:
      - ./test-files:/app/test-files:ro
      - ./cache:/app/.kreuzberg
    environment:
      - PYTHONUNBUFFERED=1
      - KREUZBERG_LOG_LEVEL=DEBUG
    command: >
      litestar --app kreuzberg._api.main:app run
      --host 0.0.0.0
      --port 8000
      --reload
```

## Configuration

### Using Configuration Files

Create `kreuzberg.toml`:

```toml
# Basic settings
force_ocr = false
ocr_backend = "tesseract"

# Chunking (requires core image or custom build)
chunk_content = true
max_chars = 1000
max_overlap = 100

# Language detection (requires core image or custom build)
auto_detect_language = true

# OCR configuration
[tesseract]
language = "eng+spa+fra"  # English, Spanish, French
psm = 6  # Uniform block of text
```

Mount the configuration:

```bash
docker run -p 8000:8000 \
  -v "$(pwd)/kreuzberg.toml:/app/kreuzberg.toml:ro" \
  -v "$(pwd)/cache:/app/.kreuzberg" \
  goldziher/kreuzberg-core:latest
```

### Environment Variables

**Cache Configuration:**

- `KREUZBERG_CACHE_DIR`: Cache directory (default: `/app/.kreuzberg`)
- `KREUZBERG_OCR_CACHE_SIZE_MB`: OCR cache size limit (default: `500`)
- `KREUZBERG_DOCUMENT_CACHE_SIZE_MB`: Document cache size limit (default: `1000`)

**Runtime Configuration:**

- `KREUZBERG_CHUNK_CONTENT`: Enable text chunking (`true`/`false`)
- `KREUZBERG_AUTO_DETECT_LANGUAGE`: Enable language detection (`true`/`false`)
- `KREUZBERG_MAX_CHARS`: Maximum characters per chunk (default: `1000`)
- `KREUZBERG_OCR_BACKEND`: OCR backend to use (`tesseract` or `none`)

**System Variables:**

- `PYTHONUNBUFFERED=1`: Ensures proper logging output
- `PYTHONDONTWRITEBYTECODE=1`: Prevents .pyc file creation

## Production Deployment

### Kubernetes

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: kreuzberg-api
  labels:
    app: kreuzberg-api
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
        image: goldziher/kreuzberg-core:latest
        ports:
        - containerPort: 8000
        volumeMounts:
        - name: cache
          mountPath: /app/.kreuzberg
        - name: config
          mountPath: /app/kreuzberg.toml
          subPath: kreuzberg.toml
          readOnly: true
        env:
        - name: PYTHONUNBUFFERED
          value: "1"
        - name: KREUZBERG_CACHE_DIR
          value: "/app/.kreuzberg"
        livenessProbe:
          httpGet:
            path: /health
            port: 8000
          initialDelaySeconds: 30
          periodSeconds: 30
        readinessProbe:
          httpGet:
            path: /health
            port: 8000
          initialDelaySeconds: 10
          periodSeconds: 10
        resources:
          requests:
            memory: "1Gi"
            cpu: "500m"
          limits:
            memory: "2Gi"
            cpu: "2000m"
      volumes:
      - name: cache
        emptyDir:
          sizeLimit: 5Gi
      - name: config
        configMap:
          name: kreuzberg-config
---
apiVersion: v1
kind: ConfigMap
metadata:
  name: kreuzberg-config
data:
  kreuzberg.toml: |
    chunk_content = true
    auto_detect_language = true
    max_chars = 1000
    ocr_backend = "tesseract"

    [tesseract]
    language = "eng+spa+fra+deu"
    psm = 6
---
apiVersion: v1
kind: Service
metadata:
  name: kreuzberg-service
spec:
  selector:
    app: kreuzberg-api
  ports:
  - port: 80
    targetPort: 8000
  type: ClusterIP
```

### With Nginx Reverse Proxy

```nginx
upstream kreuzberg {
    server localhost:8000;
    # Add more servers for load balancing
    # server localhost:8001;
    # server localhost:8002;
}

server {
    listen 80;
    server_name api.example.com;

    location / {
        proxy_pass http://kreuzberg;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;

        # File upload settings
        client_max_body_size 100M;
        proxy_read_timeout 300s;
        proxy_connect_timeout 60s;
        proxy_send_timeout 300s;
    }

    location /health {
        proxy_pass http://kreuzberg/health;
        access_log off;
    }

    # Optional: Basic rate limiting
    location /extract {
        limit_req zone=api burst=10 nodelay;
        proxy_pass http://kreuzberg/extract;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}

# Rate limiting configuration (add to http block)
http {
    limit_req_zone $binary_remote_addr zone=api:10m rate=60r/m;
}
```

## Resource Requirements

| Configuration           | CPU      | Memory | Storage | Use Case                   |
| ----------------------- | -------- | ------ | ------- | -------------------------- |
| Base (basic extraction) | 1+ cores | 512MB+ | 1GB     | Simple document processing |
| Base (with cache)       | 1+ cores | 1GB+   | 2GB     | Moderate throughput        |
| Core (full features)    | 2+ cores | 1.5GB+ | 3GB     | Advanced processing        |
| Core (high throughput)  | 4+ cores | 3GB+   | 5GB     | Enterprise workloads       |

## Troubleshooting

### Common Issues

#### Container Fails to Start

```bash
# Check logs for startup errors
docker logs <container-name>

# Common causes:
# - Insufficient memory allocation
# - Port already in use
# - Invalid configuration file
```

#### Permission Issues with Cache Directory

```bash
# Fix cache directory permissions
docker run --rm -v "$(pwd)/cache:/app/.kreuzberg" --user root \
  goldziher/kreuzberg:latest \
  chown -R 999:999 /app/.kreuzberg
```

#### Out of Memory Errors

```bash
# Solution 1: Increase Docker memory limits
docker run -p 8000:8000 -m 2g goldziher/kreuzberg-core:latest

# Solution 2: Reduce cache sizes
docker run -p 8000:8000 \
  -e KREUZBERG_OCR_CACHE_SIZE_MB=200 \
  -e KREUZBERG_DOCUMENT_CACHE_SIZE_MB=300 \
  goldziher/kreuzberg-core:latest
```

#### Feature Not Available Errors

```bash
# Error: "chunking requires semantic-text-splitter"
# Solution: Use core image or build custom image with required dependency

# Error: "language detection requires fast-langdetect"
# Solution: Use core image or add the dependency to custom build
```

### Debugging

```bash
# Interactive shell access
docker exec -it <container-id> /bin/bash

# Test extraction directly
docker exec <container-id> python3 -c "
from kreuzberg import extract_file_sync
result = extract_file_sync('/app/test-files/sample.pdf')
print(f'Extracted {len(result.content)} characters')
print(result.content[:200])
"

# Check available features
docker exec <container-id> python3 -c "
import kreuzberg
print(f'Kreuzberg version: {kreuzberg.__version__}')

# Test imports to see what's available
try:
    import semantic_text_splitter
    print('✅ Chunking available')
except ImportError:
    print('❌ Chunking not available')

try:
    import fast_langdetect
    print('✅ Language detection available')
except ImportError:
    print('❌ Language detection not available')
"
```

### Performance Monitoring

```bash
# Monitor resource usage
docker stats <container-name>

# View API metrics
curl http://localhost:8000/metrics  # If OpenTelemetry is enabled

# Health check endpoint
curl http://localhost:8000/health
```

## Security Considerations

- Runs as non-root user (`appuser:999`) by default
- No external API calls or cloud dependencies
- All processing happens locally within the container
- Use read-only mounts for configuration files (`:ro`)
- Consider adding authentication middleware for production deployments
- Regular security updates through base image rebuilds
- Isolate containers using Docker networks
- Use secrets management for sensitive configuration
