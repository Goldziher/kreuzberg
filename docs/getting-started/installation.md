# Installation

Kreuzberg is available for Python, TypeScript/Node.js, and as a standalone CLI via Rust/Cargo or Homebrew.

## Choose Your Platform

=== "Python"

    **Requirements**: Python 3.10 or later

    ### Basic Installation

    ```bash
    pip install kreuzberg
    ```

    ### With All Features

    ```bash
    pip install "kreuzberg[all]"
    ```

    ### Individual Features

    ```bash
    # API server
    pip install "kreuzberg[api]"

    # CLI tools
    pip install "kreuzberg[cli]"

    # PDF encryption support (AES)
    pip install "kreuzberg[crypto]"

    # EasyOCR backend
    pip install "kreuzberg[easyocr]"

    # PaddleOCR backend
    pip install "kreuzberg[paddleocr]"

    # Vision-based table extraction
    pip install "kreuzberg[vision-tables]"

    # Language detection
    pip install "kreuzberg[langdetect]"
    ```

    !!! warning "Python 3.14 Compatibility"
        The `easyocr` and `paddleocr` extras are currently **not supported on Python 3.14** due to upstream compatibility issues with EasyOCR, PaddleOCR, and PyTorch.

        These features are available on Python 3.10-3.13. If you need OCR functionality on Python 3.14, use the built-in Tesseract backend instead (no extra installation required).

        Support for Python 3.14 will be added once the upstream packages release compatible versions.

    ### Verify Installation

    ```python
    from kreuzberg import extract_file_sync, __version__
    print(f"Kreuzberg {__version__} installed successfully!")
    ```

=== "TypeScript"

    **Requirements**: Node.js 18 or later

    ### npm

    ```bash
    npm install @goldziher/kreuzberg
    ```

    ### yarn

    ```bash
    yarn add @goldziher/kreuzberg
    ```

    ### pnpm

    ```bash
    pnpm add @goldziher/kreuzberg
    ```

    ### Verify Installation

    ```typescript
    import { extractFileSync } from '@goldziher/kreuzberg';
    console.log('Kreuzberg installed successfully!');
    ```

=== "CLI (Homebrew)"

    **Requirements**: macOS or Linux

    ### Install via Homebrew

    ```bash
    brew install kreuzberg
    ```

    ### Verify Installation

    ```bash
    kreuzberg --version
    ```

=== "CLI (Cargo)"

    **Requirements**: Rust 1.75 or later

    ### Install via Cargo

    ```bash
    cargo install kreuzberg-cli
    ```

    ### Verify Installation

    ```bash
    kreuzberg --version
    ```

## System Dependencies

Kreuzberg requires some system libraries for full functionality:

### Tesseract OCR (Optional)

Required for OCR support with the Tesseract backend.

=== "macOS"

    ```bash
    brew install tesseract
    ```

=== "Ubuntu/Debian"

    ```bash
    sudo apt-get install tesseract-ocr
    ```

=== "RHEL/CentOS/Fedora"

    ```bash
    sudo dnf install tesseract
    ```

=== "Windows"

    Download from [GitHub releases](https://github.com/UB-Mannheim/tesseract/wiki)

### Pandoc (Optional)

Required for extracting certain document formats (DOCX, ODT, etc.) via Pandoc.

=== "macOS"

    ```bash
    brew install pandoc
    ```

=== "Ubuntu/Debian"

    ```bash
    sudo apt-get install pandoc
    ```

=== "RHEL/CentOS/Fedora"

    ```bash
    sudo dnf install pandoc
    ```

=== "Windows"

    Download from [pandoc.org](https://pandoc.org/installing.html)

### LibreOffice (Optional)

Required for legacy MS Office formats (.doc, .ppt).

=== "macOS"

    ```bash
    brew install libreoffice
    ```

=== "Ubuntu/Debian"

    ```bash
    sudo apt-get install libreoffice
    ```

=== "RHEL/CentOS/Fedora"

    ```bash
    sudo dnf install libreoffice
    ```

## Docker

Pre-built Docker images are available with all dependencies included:

```bash
# Core image with Tesseract
docker pull goldziher/kreuzberg:latest

# With EasyOCR
docker pull goldziher/kreuzberg:latest-easyocr

# With PaddleOCR
docker pull goldziher/kreuzberg:latest-paddle

# With vision-based table extraction
docker pull goldziher/kreuzberg:latest-vision-tables

# All features
docker pull goldziher/kreuzberg:latest-all
```

### Run API Server

```bash
docker run -p 8000:8000 goldziher/kreuzberg:latest
```

## Next Steps

- [Quick Start Guide](quickstart.md) - Get started with your first extraction
- [Contributing](../contributing.md) - Learn how to contribute
