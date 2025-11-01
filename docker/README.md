# Kreuzberg Docker Images

This directory contains Dockerfile variants for building Kreuzberg Docker images with different feature sets.

## Base Image

Both variants use **Debian 13 (Trixie) slim** - the latest stable Debian release for optimal package availability and security updates.

## Image Variants

### 1. Core Image (`Dockerfile.core`)

**Size:** ~1.0-1.3GB
**Base:** debian:trixie-slim
**Features:** PDF, DOCX, PPTX, images, HTML, XML, text, Excel, email, Pandoc formats
**OCR:** Tesseract (12 languages)
**Missing:** LibreOffice (no legacy .doc, .ppt support)

**When to use:**
- Production deployments where image size matters
- When you don't need legacy MS Office format support (.doc, .ppt)
- Cloud environments with size/bandwidth constraints
- Kubernetes deployments with frequent pod scaling

**Build command:**
```bash
docker build -f docker/Dockerfile.core -t kreuzberg:core .
```

### 2. Full Image (`Dockerfile.full`)

**Size:** ~1.5-2.1GB
**Base:** debian:trixie-slim
**Features:** All core features + LibreOffice for legacy Office formats
**OCR:** Tesseract (12 languages)
**Includes:** LibreOffice 25.8.2 for .doc, .ppt conversion

**When to use:**
- Need to process legacy MS Office files (.doc, .ppt)
- Complete document intelligence pipeline
- Development and testing environments
- When image size is not a constraint

**Build command:**
```bash
docker build -f docker/Dockerfile.full -t kreuzberg:full .
```

## Size Comparison

| Component | Core | Full | Difference |
|-----------|------|------|------------|
| Base (trixie-slim) | ~120MB | ~120MB | - |
| Tesseract + 12 langs | ~250MB | ~250MB | - |
| Rust binary | ~80MB | ~80MB | - |
| Pandoc | ~50MB | ~50MB | - |
| pdfium | ~30MB | ~30MB | - |
| System libraries | ~100MB | ~100MB | - |
| **LibreOffice** | - | **~500-800MB** | **+500-800MB** |
| **Total (approx)** | **~1.0-1.3GB** | **~1.5-2.1GB** | **~500-800MB** |

## Default Image

The root `Dockerfile` is a symlink to `Dockerfile.full` for backward compatibility and complete feature support by default.

## Multi-Architecture Support

Both images support:
- `linux/amd64` (x86_64)
- `linux/arm64` (aarch64)

Architecture-specific binaries (Pandoc, LibreOffice, pdfium) are automatically selected during build.

## Usage Modes

All images support three execution modes via ENTRYPOINT:

### 1. API Server (default)
```bash
docker run -p 8000:8000 kreuzberg:core
# or override host/port:
docker run -p 8000:8000 kreuzberg:core serve --host 0.0.0.0 --port 8000
```

### 2. CLI Mode
```bash
docker run -v $(pwd):/data kreuzberg:core extract /data/document.pdf
docker run -v $(pwd):/data kreuzberg:core detect /data/file.bin
docker run -v $(pwd):/data kreuzberg:core batch /data/*.pdf
```

### 3. MCP Server Mode
```bash
docker run kreuzberg:core mcp
```

## Testing

Test scripts are provided to verify both image variants:

```bash
# Test core image
IMAGE_NAME=kreuzberg:core ./scripts/test_docker.sh

# Test full image
IMAGE_NAME=kreuzberg:full ./scripts/test_docker.sh
```

## GitHub Actions

The `.github/workflows/docker.yml` workflow builds and publishes both variants:
- `kreuzberg:v4-core` - Core image without LibreOffice
- `kreuzberg:v4-full` - Full image with LibreOffice
- `kreuzberg:v4`, `kreuzberg:latest` - Aliases for full image

## Recommendations

**Choose Core if:**
- ✅ You don't need legacy .doc/.ppt support
- ✅ Image size is a concern
- ✅ Faster pull/deployment times matter
- ✅ Cloud costs are sensitive to egress/storage

**Choose Full if:**
- ✅ You need complete Office format support
- ✅ Processing legacy documents (.doc, .ppt)
- ✅ Image size is not a constraint
- ✅ You want "batteries included" experience
