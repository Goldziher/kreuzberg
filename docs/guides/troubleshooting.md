# Troubleshooting

Common issues and solutions when using Kreuzberg.

## Installation Issues

??? question "Kreuzberg package not found"

    **Error**: `ModuleNotFoundError: No module named 'kreuzberg'` (Python) or similar

    **Solution**:

    ```bash
    # Python
    pip install kreuzberg

    # TypeScript
    npm install kreuzberg

    # Rust - add to Cargo.toml
    [dependencies]
    kreuzberg = "4.0"

    # Ruby
    gem install kreuzberg
    ```

    Verify installation:

    ```bash
    # Python
    python -c "import kreuzberg; print(kreuzberg.__version__)"

    # TypeScript
    node -e "const k = require('kreuzberg'); console.log('OK')"

    # Rust
    cargo check

    # Ruby
    ruby -e "require 'kreuzberg'; puts Kreuzberg::VERSION"
    ```

??? question "Build errors during installation (Python/TypeScript/Ruby)"

    **Error**: Compilation errors when installing binary packages

    **Solution**:

    1. **Ensure compatible Python version**: Python 3.10-3.14 required
        ```bash
        python --version  # Check version
        ```

    2. **Install build dependencies** (Linux):
        ```bash
        # Ubuntu/Debian
        sudo apt-get install build-essential python3-dev

        # RHEL/CentOS/Fedora
        sudo dnf install gcc python3-devel
        ```

    3. **Update pip/setuptools**:
        ```bash
        pip install --upgrade pip setuptools wheel
        ```

    4. **Try pre-built wheels**:
        ```bash
        # Force binary wheel installation
        pip install --only-binary :all: kreuzberg
        ```

## Dependency Issues

??? question "Tesseract not found"

    **Error**: `MissingDependencyError: tesseract not found in PATH`

    **Solution**: Install Tesseract OCR

    === "macOS"

        ```bash
        brew install tesseract

        # Verify installation
        tesseract --version
        which tesseract
        ```

    === "Ubuntu/Debian"

        ```bash
        sudo apt-get update
        sudo apt-get install tesseract-ocr

        # Verify installation
        tesseract --version
        which tesseract
        ```

    === "RHEL/CentOS/Fedora"

        ```bash
        sudo dnf install tesseract

        # Verify installation
        tesseract --version
        ```

    === "Windows"

        1. Download from [UB Mannheim](https://github.com/UB-Mannheim/tesseract/wiki)
        2. Install to default location: `C:\Program Files\Tesseract-OCR`
        3. Add to PATH: `C:\Program Files\Tesseract-OCR`
        4. Verify: `tesseract --version`

??? question "LibreOffice not found"

    **Error**: `MissingDependencyError: libreoffice not found in PATH`

    **Solution**: Install LibreOffice (required for .doc and .ppt files)

    === "macOS"

        ```bash
        brew install --cask libreoffice

        # Verify installation
        soffice --version
        ```

    === "Ubuntu/Debian"

        ```bash
        sudo apt-get install libreoffice

        # Verify installation
        soffice --version
        ```

    === "RHEL/CentOS/Fedora"

        ```bash
        sudo dnf install libreoffice

        # Verify installation
        soffice --version
        ```

    === "Windows"

        1. Download from [LibreOffice.org](https://www.libreoffice.org/download/download/)
        2. Install to default location
        3. Add to PATH: `C:\Program Files\LibreOffice\program`
        4. Verify: `soffice --version`

    **Alternative**: Use modern Office formats (.docx, .pptx) instead, which don't require LibreOffice.

??? question "Pandoc not found"

    **Error**: `MissingDependencyError: pandoc not found in PATH`

    **Solution**: Install Pandoc (required for some document conversions)

    === "macOS"

        ```bash
        brew install pandoc

        # Verify installation
        pandoc --version
        ```

    === "Ubuntu/Debian"

        ```bash
        sudo apt-get install pandoc

        # Verify installation
        pandoc --version
        ```

    === "RHEL/CentOS/Fedora"

        ```bash
        sudo dnf install pandoc

        # Verify installation
        pandoc --version
        ```

    === "Windows"

        1. Download from [pandoc.org](https://pandoc.org/installing.html)
        2. Install to default location
        3. Verify: `pandoc --version`

## Extraction Issues

??? question "Unsupported file format"

    **Error**: `ParsingError: Unsupported file format` or `ValidationError: Unknown MIME type`

    **Solution**:

    1. **Check file extension**: Kreuzberg supports 118+ formats. Verify your file extension is recognized.

    2. **Explicitly specify MIME type** when using `extract_bytes`:
        ```python
        result = extract_bytes_sync(
            data,
            mime_type="application/pdf"  # Explicit MIME type
        )
        ```

    3. **Check file corruption**: Verify the file opens in its native application.

    4. **See supported formats**: [Extraction guide](extraction.md#supported-formats)

??? question "Empty extraction result"

    **Problem**: `result.content` is empty or contains only whitespace

    **Solutions**:

    1. **For images/scanned PDFs**: Enable OCR
        ```python
        config = ExtractionConfig(
            ocr=OcrConfig(backend="tesseract")
        )
        result = extract_file_sync("scanned.pdf", config=config)
        ```

    2. **For password-protected PDFs**: Provide password
        ```python
        config = ExtractionConfig(
            pdf=PdfConfig(password="your_password")
        )
        ```

    3. **Check file validity**: Ensure file is not corrupted
        ```bash
        # Try opening in native application
        open document.pdf  # macOS
        xdg-open document.pdf  # Linux
        ```

    4. **Check metadata**: Some files may have metadata but no content
        ```python
        print(f"Metadata: {result.metadata}")
        print(f"Tables: {len(result.tables)}")
        ```

??? question "Password-protected PDF not working"

    **Error**: `ParsingError: Failed to open password-protected PDF`

    **Solutions**:

    1. **Provide correct password**:
        ```python
        config = ExtractionConfig(
            pdf=PdfConfig(password="your_password")
        )
        result = extract_file_sync("protected.pdf", config=config)
        ```

    2. **Try multiple passwords**:
        ```python
        config = ExtractionConfig(
            pdf=PdfConfig(password=["password1", "password2", "password3"])
        )
        ```

    3. **For AES-encrypted PDFs**: Install crypto extra (Python only)
        ```bash
        pip install "kreuzberg[crypto]"
        ```

    4. **Check encryption type**: RC4 is supported in base package, AES requires `crypto` extra.

??? question "Failed to parse document"

    **Error**: `ParsingError: Failed to parse document`

    **Solutions**:

    1. **Check file corruption**: Try opening the file in its native application.

    2. **Update Kreuzberg**: Ensure you have the latest version
        ```bash
        pip install --upgrade kreuzberg  # Python
        npm update kreuzberg  # TypeScript
        cargo update kreuzberg  # Rust
        gem update kreuzberg  # Ruby
        ```

    3. **Try different extraction method**: If using bytes, try file path instead
        ```python
        # Instead of extract_bytes, try extract_file
        result = extract_file_sync("document.pdf")
        ```

    4. **Report issue**: If the file should be supported, [report a bug](https://github.com/Goldziher/kreuzberg/issues)

## Performance Issues

??? question "Extraction is very slow"

    **Problem**: Processing takes much longer than expected

    **Solutions**:

    1. **Use batch processing** for multiple files:
        ```python
        # Instead of loop
        results = batch_extract_files_sync(files, config=config)
        ```

    2. **Reduce OCR DPI** if using OCR:
        ```python
        config = ExtractionConfig(
            ocr=OcrConfig(backend="tesseract"),
            pdf=PdfConfig(dpi=150)  # Lower DPI = faster
        )
        ```

    3. **Disable quality processing** if not needed:
        ```python
        config = ExtractionConfig(
            enable_quality_processing=False
        )
        ```

    4. **Use sync methods** for single files (async has overhead):
        ```python
        # Sync is faster for single files
        result = extract_file_sync("document.pdf")
        ```

    5. **Switch OCR backend**: PaddleOCR is faster than EasyOCR
        ```python
        config = ExtractionConfig(
            ocr=OcrConfig(backend="paddleocr", use_gpu=True)
        )
        ```

??? question "Out of memory errors"

    **Error**: `MemoryError` or process killed

    **Solutions**:

    1. **Reduce OCR DPI** for large PDFs:
        ```python
        config = ExtractionConfig(
            ocr=OcrConfig(backend="tesseract"),
            pdf=PdfConfig(dpi=150)  # Use less memory
        )
        ```

    2. **Disable caching** to save memory:
        ```python
        config = ExtractionConfig(
            use_cache=False
        )
        ```

    3. **Process in smaller batches**:
        ```python
        # Instead of all at once
        for batch in chunks(files, 10):
            results = batch_extract_files_sync(batch)
        ```

    4. **Increase system memory**: OCR and large PDFs are memory-intensive.

    5. **Use Tesseract backend**: More memory-efficient than deep learning OCR
        ```python
        config = ExtractionConfig(
            ocr=OcrConfig(backend="tesseract")  # Uses less memory
        )
        ```

## OCR Issues

See also: [OCR Troubleshooting](ocr.md#troubleshooting)

??? question "Poor OCR accuracy"

    **Problem**: Extracted text has many errors or is garbled

    **Solutions**:

    1. **Increase DPI** for better quality:
        ```python
        config = ExtractionConfig(
            ocr=OcrConfig(backend="tesseract"),
            pdf=PdfConfig(dpi=600)  # Higher quality
        )
        ```

    2. **Use deep learning OCR** for better accuracy:
        ```python
        # EasyOCR has best accuracy
        config = ExtractionConfig(
            ocr=OcrConfig(backend="easyocr", language="en")
        )
        ```

    3. **Specify correct language**:
        ```python
        config = ExtractionConfig(
            ocr=OcrConfig(backend="tesseract", language="deu")  # German
        )
        ```

    4. **Check image quality**: Low-quality scans will have poor OCR results.

??? question "OCR language not found"

    **Error**: `Failed to initialize tesseract with language 'deu'`

    **Solution**: Install language data

    === "macOS"

        ```bash
        brew install tesseract-lang

        # Verify languages
        tesseract --list-langs
        ```

    === "Ubuntu/Debian"

        ```bash
        # Install specific language
        sudo apt-get install tesseract-ocr-deu  # German
        sudo apt-get install tesseract-ocr-fra  # French
        sudo apt-get install tesseract-ocr-spa  # Spanish

        # Verify languages
        tesseract --list-langs
        ```

    === "RHEL/CentOS/Fedora"

        ```bash
        sudo dnf install tesseract-langpack-deu

        # Verify languages
        tesseract --list-langs
        ```

## Platform-Specific Issues

??? question "TypeScript: Cannot find module 'kreuzberg'"

    **Error**: `Cannot find module 'kreuzberg'` or `ERR_MODULE_NOT_FOUND`

    **Solution**:

    1. **Verify installation**:
        ```bash
        npm list kreuzberg
        # or
        pnpm list kreuzberg
        ```

    2. **Reinstall package**:
        ```bash
        npm install kreuzberg
        # or
        pnpm add kreuzberg
        ```

    3. **Check Node.js version**: Requires Node.js 18+
        ```bash
        node --version  # Should be >= 18.0.0
        ```

    4. **Check import syntax**:
        ```typescript
        // ESM
        import { extractFileSync } from 'kreuzberg';

        // CommonJS
        const { extractFileSync } = require('kreuzberg');
        ```

??? question "Rust: Linker errors during build"

    **Error**: `error: linking with 'cc' failed`

    **Solution**:

    1. **Install C compiler** (required for native dependencies):
        ```bash
        # Ubuntu/Debian
        sudo apt-get install build-essential

        # macOS (Xcode command line tools)
        xcode-select --install

        # Windows (Visual Studio Build Tools)
        # Install from https://visualstudio.microsoft.com/downloads/
        ```

    2. **Update Rust**:
        ```bash
        rustup update
        ```

    3. **Clean and rebuild**:
        ```bash
        cargo clean
        cargo build --release
        ```

??? question "Windows: PATH issues"

    **Problem**: Commands like `tesseract`, `soffice`, or `pandoc` not found

    **Solution**:

    1. **Add to System PATH**:
        - Open "Environment Variables" in System Properties
        - Edit "Path" variable
        - Add directory containing the executable:
            - Tesseract: `C:\Program Files\Tesseract-OCR`
            - LibreOffice: `C:\Program Files\LibreOffice\program`
            - Pandoc: `C:\Program Files\Pandoc`

    2. **Restart terminal** after changing PATH

    3. **Verify**:
        ```powershell
        tesseract --version
        soffice --version
        pandoc --version
        ```

## Getting Help

If your issue isn't covered here:

1. **Check existing issues**: [GitHub Issues](https://github.com/Goldziher/kreuzberg/issues)
2. **Search documentation**: Use the search bar at the top of this page
3. **Report a bug**: [Create a new issue](https://github.com/Goldziher/kreuzberg/issues/new)

When reporting issues, include:

- Kreuzberg version
- Platform (OS, Python/Node/Rust/Ruby version)
- Complete error message
- Minimal reproducible example
- File format causing the issue (if applicable)

## Next Steps

- [Extraction Basics](extraction.md) - Core extraction API
- [OCR Guide](ocr.md) - OCR setup and configuration
- [Configuration](configuration.md) - All configuration options
- [Installation](../getting-started/installation.md) - Installation guide
