# Kreuzberg Docker Images

[![GitHub](https://img.shields.io/badge/GitHub-Goldziher%2Fkreuzberg-blue)](https://github.com/Goldziher/kreuzberg)
[![PyPI](https://badge.fury.io/py/kreuzberg.svg)](https://badge.fury.io/py/kreuzberg)
[![Documentation](https://img.shields.io/badge/docs-GitHub_Pages-blue)](https://goldziher.github.io/kreuzberg/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://github.com/Goldziher/kreuzberg/blob/main/LICENSE)

High-performance Python library for text extraction from documents, available as Docker images.

**Source Code**: [github.com/Goldziher/kreuzberg](https://github.com/Goldziher/kreuzberg)

## Quick Start

```bash
docker run -p 8000:8000 goldziher/kreuzberg:latest
```

## Available Tags

- `latest` - Latest stable release with API server and Tesseract OCR
- `X.Y.Z` - Specific version (e.g., `3.0.0`)
- `X.Y.Z-easyocr` - With EasyOCR support
- `X.Y.Z-paddle` - With PaddleOCR support
- `X.Y.Z-gmft` - With GMFT table extraction
- `X.Y.Z-all` - With all optional dependencies

## Usage

### Extract Files via API

```bash
# Single file
curl -X POST http://localhost:8000/extract \
  -F "data=@document.pdf"

# Multiple files
curl -X POST http://localhost:8000/extract \
  -F "data=@document1.pdf" \
  -F "data=@document2.docx"
```

### Docker Compose

```yaml
version: '3.8'

services:
  kreuzberg:
    image: goldziher/kreuzberg:latest
    ports:
      - "8000:8000"
    restart: unless-stopped
```

## Features

- **🚀 High Performance**: Optimized for speed and efficiency
- **📄 Multiple Formats**: PDF, DOCX, images, HTML, and more
- **🔍 OCR Support**: Built-in Tesseract, optional EasyOCR/PaddleOCR
- **📊 Table Extraction**: Extract tables with GMFT
- **🔒 Secure**: Runs as non-root user, no external API calls
- **📦 Ready to Use**: Pre-configured API server

## Documentation

- **[GitHub Repository](https://github.com/Goldziher/kreuzberg)** - Source code and issue tracking
- **[Full Documentation](https://goldziher.github.io/kreuzberg/)** - Complete user guide and API reference
- **[API Documentation](https://goldziher.github.io/kreuzberg/user-guide/api-server/)** - REST API endpoints and usage
- **[Docker Guide](https://goldziher.github.io/kreuzberg/user-guide/docker/)** - Detailed Docker usage guide

## Support

- **Issues**: [github.com/Goldziher/kreuzberg/issues](https://github.com/Goldziher/kreuzberg/issues)
- **Discussions**: [github.com/Goldziher/kreuzberg/discussions](https://github.com/Goldziher/kreuzberg/discussions)
- **Discord**: [Join our community](https://discord.gg/pXxagNK2zN)

## Contributing

Contributions are welcome! See our [Contributing Guide](https://github.com/Goldziher/kreuzberg/blob/main/docs/contributing.md).

## License

MIT License - see [LICENSE](https://github.com/Goldziher/kreuzberg/blob/main/LICENSE) for details.

______________________________________________________________________

Made with ❤️ by the [Kreuzberg contributors](https://github.com/Goldziher/kreuzberg/graphs/contributors)
