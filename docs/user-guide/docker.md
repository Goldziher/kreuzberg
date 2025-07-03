# Docker

Kreuzberg provides official Docker images for easy deployment and containerized usage.

## Available Images

Docker images are available on [Docker Hub](https://hub.docker.com/r/goldziher/kreuzberg):

- `goldziher/kreuzberg:latest` - Core image with API server and Tesseract OCR
- `goldziher/kreuzberg:latest-easyocr` - With EasyOCR support
- `goldziher/kreuzberg:latest-paddle` - With PaddleOCR support
- `goldziher/kreuzberg:latest-gmft` - With GMFT table extraction
- `goldziher/kreuzberg:latest-all` - With all optional dependencies

> **Note**: Specific version tags are also available (e.g., `v3.4.0`, `v3.4.0-easyocr`)

## Quick Start

### Running the API Server

```bash
# Pull the latest image
docker pull goldziher/kreuzberg:latest

# Run the API server
docker run -p 8000:8000 goldziher/kreuzberg:latest
```

The API server will be available at `http://localhost:8000`.

### Extract Files

```bash
# Extract a single file
curl -X POST http://localhost:8000/extract \
  -F "data=@document.pdf"

# Extract multiple files
curl -X POST http://localhost:8000/extract \
  -F "data=@document1.pdf" \
  -F "data=@document2.docx"
```

## Docker Compose

Create a `docker-compose.yml`:

```yaml
services:
  kreuzberg:
    image: goldziher/kreuzberg:latest
    ports:
      - "8000:8000"
    environment:
      - PYTHONUNBUFFERED=1
    restart: unless-stopped
```

Run with:

```bash
docker-compose up -d
```

## Using Different OCR Engines

### EasyOCR

```bash
docker run -p 8000:8000 goldziher/kreuzberg:latest-easyocr
```

### PaddleOCR

```bash
docker run -p 8000:8000 goldziher/kreuzberg:latest-paddle
```

### All Features

```bash
docker run -p 8000:8000 goldziher/kreuzberg:latest-all
```

## Building Custom Images

If you need a custom configuration, you can build your own image:

```dockerfile
FROM goldziher/kreuzberg:latest

# Add custom dependencies
RUN pip install your-custom-package

# Add custom configuration
COPY custom_config.py /app/

# Use custom startup command
CMD ["python", "custom_config.py"]
```

## Image Details

### Base Image

- Based on `python:3.13-bookworm` (requires Python 3.10+)
- Includes system dependencies: `pandoc`, `tesseract-ocr`
- Runs as non-root user `appuser`
- Exposes port 8000

### Included Dependencies

All images include:

- Kreuzberg core library
- Litestar API framework
- Tesseract OCR
- Pandoc for document conversion

Additional dependencies by variant:

- **easyocr**: EasyOCR deep learning models
- **paddle**: PaddleOCR and PaddlePaddle
- **gmft**: GMFT for table extraction
- **all**: All optional dependencies

### Health Check

All Docker images include a health check endpoint:

```bash
# Check API health
curl http://localhost:8000/health
```

Returns a JSON response with service status and version information.

### Observability

The Docker images include built-in OpenTelemetry instrumentation via Litestar:

- **Tracing**: Automatic request/response tracing
- **Metrics**: Performance and usage metrics
- **Logging**: Structured JSON logging

Configure via standard OpenTelemetry environment variables:

```bash
docker run -p 8000:8000 \
  -e OTEL_SERVICE_NAME=kreuzberg-api \
  -e OTEL_EXPORTER_OTLP_ENDPOINT=http://your-collector:4317 \
  goldziher/kreuzberg:latest
```

### Environment Variables

- `PYTHONUNBUFFERED=1` - Ensures proper logging output
- `PYTHONDONTWRITEBYTECODE=1` - Prevents .pyc file creation
- `UV_LINK_MODE=copy` - Optimizes package installation

## Production Deployment

### With nginx Reverse Proxy

```nginx
server {
    listen 80;
    server_name api.example.com;

    location / {
        proxy_pass http://localhost:8000;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;

        # File upload settings
        client_max_body_size 100M;
        proxy_read_timeout 300s;
    }

    # Health check endpoint
    location /health {
        proxy_pass http://localhost:8000/health;
        access_log off;
    }
}
```

### Kubernetes Deployment

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
        livenessProbe:
          httpGet:
            path: /health
            port: 8000
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /health
            port: 8000
          initialDelaySeconds: 5
          periodSeconds: 5
        env:
        - name: OTEL_SERVICE_NAME
          value: "kreuzberg-api"
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
    - port: 80
      targetPort: 8000
  type: LoadBalancer
```

## Resource Requirements

### Minimum Requirements

- **CPU**: 1 core
- **Memory**: 512MB
- **Storage**: 1GB (more for OCR models)

### Recommended for Production

- **CPU**: 2+ cores
- **Memory**: 2GB+ (4GB+ for EasyOCR/PaddleOCR)
- **Storage**: 5GB+ (depends on OCR models)

### OCR Model Sizes

- **Tesseract**: ~100MB
- **EasyOCR**: ~64MB-2.5GB per language
- **PaddleOCR**: ~400MB

## Troubleshooting

### Container Logs

```bash
docker logs <container-id>
```

### Shell Access

```bash
docker exec -it <container-id> /bin/bash
```

### Common Issues

1. **Out of Memory**: Increase Docker memory allocation or use a smaller OCR engine
1. **Slow Startup**: First run downloads OCR models; subsequent runs are faster
1. **Permission Denied**: Ensure mounted volumes have correct permissions

## Security Considerations

- Runs as non-root user by default
- No external API calls or cloud dependencies
- Process files locally within the container
- Consider adding authentication for production use
- Use volume mounts carefully to limit file system access
