# Installation

Kreuzberg is a modular document intelligence framework with a core package and optional components for specialized functionality.

## System Dependencies

### Pandoc

Pandoc is the foundation of Kreuzberg's universal document conversion capabilities. This **required** system dependency enables reliable extraction across diverse document formats. Install Pandoc for your platform:

#### Ubuntu/Debian

```shell
sudo apt-get install pandoc
```

#### macOS

```shell
brew install pandoc
```

#### Windows

```shell
choco install -y pandoc
```

## Kreuzberg Core Package

The Kreuzberg core package can be installed using pip with:

```shell
pip install kreuzberg
```

!!! note "Python Compatibility"

    Kreuzberg runs on Python 3.10 through 3.14. The core library fully supports 3.14, but some optional extras (EasyOCR, PaddleOCR, entity extraction via spaCy) remain unavailable until their upstream wheels add 3.14 support.

## Optional Features

### OCR

OCR is an optional feature for extracting text from images and non-searchable PDFs. Kreuzberg supports multiple OCR backends. To understand the differences between these backends, please read the [OCR Configuration documentation](../user-guide/ocr-configuration.md).

#### Tesseract OCR

Tesseract OCR is built into Kreuzberg and doesn't require additional Python packages. However, you must install Tesseract 5.0 or higher on your system:

##### Ubuntu/Debian

```shell
sudo apt-get install tesseract-ocr
```

##### macOS

```shell
brew install tesseract
```

##### Windows

```shell
choco install -y tesseract
```

!!! note "Language Support"

    Tesseract includes English language support by default. Kreuzberg Docker images come pre-configured with 12 common business languages: English, Spanish, French, German, Italian, Portuguese, Chinese (Simplified & Traditional), Japanese, Arabic, Russian, and Hindi.

    For local installations requiring additional languages, you must install the appropriate language data files:

    - **Ubuntu/Debian**: `sudo apt-get install tesseract-ocr-deu` (for German)
    - **macOS**: `brew install tesseract-lang` (includes all languages)
    - **Windows**: Download language files manually to the Tesseract `tessdata` directory:

    ```powershell
    # For German language support on Windows
    $tessDataDir = "C:\Program Files\Tesseract-OCR\tessdata"
    Invoke-WebRequest -Uri "https://github.com/tesseract-ocr/tessdata/raw/main/deu.traineddata" -OutFile "$tessDataDir\deu.traineddata"

    # Verify installation
    tesseract --list-langs
    ```

    For more details on language installation and configuration, refer to the [Tesseract documentation](https://tesseract-ocr.github.io/tessdoc/Installation.html).

#### EasyOCR

EasyOCR is a Python-based OCR backend with wide language support and strong performance.

```shell
pip install "kreuzberg[easyocr]"
```

!!! warning

    EasyOCR wheels are currently available only for Python 3.13 and below. On Python 3.14 this extra is skipped until upstream support lands.

#### PaddleOCR

PaddleOCR is particularly strong for Chinese and other Asian languages. It requires additional system dependencies for OpenCV support:

##### System Dependencies

```shell
# Ubuntu/Debian
sudo apt-get install libgl1 libglib2.0-0

# macOS
# OpenGL is typically included; if needed:
brew install glfw
```

OpenGL libraries are typically included with graphics drivers on Windows.

##### Python Package

```shell
pip install "kreuzberg[paddleocr]"
```

!!! warning

    PaddleOCR/PaddlePaddle do not yet ship Python 3.14 wheels. Installation on 3.14 will skip this extra until the upstream projects add support.

### Chunking

Chunking is an optional feature - useful for RAG applications among others. Kreuzberg uses the excellent `semantic-text-splitter` package for chunking. To install Kreuzberg with chunking support, you can use:

```shell
pip install "kreuzberg[chunking]"
```

### Table Extraction

!!! warning "GMFT Deprecation"

    The GMFT-powered table extraction pipeline is deprecated and will be removed in Kreuzberg v4.0. The upcoming release ships a native TATR-based `TableExtractionConfig` that replaces GMFT end-to-end.

If you need to keep using the legacy GMFT integration in the current series, install the extra explicitly:

```shell
pip install "kreuzberg[gmft]"
```

`GMFTConfig` will emit a `FutureWarning` whenever it is instantiated to remind you to migrate. Plan to switch to the new table extraction configuration before upgrading to v4.0.

### Language Detection

Language detection is an optional feature that automatically detects the language of extracted text. It uses the [fast-langdetect](https://github.com/LlmKira/fast-langdetect) package. To install Kreuzberg with language detection support, you can use:

```shell
pip install "kreuzberg[langdetect]"
```

### Document Classification

For automatic document type detection (invoice, contract, receipt, etc.), install the document classification extra:

```shell
pip install "kreuzberg[document-classification]"
```

This feature uses Google Translate for multi-language support and requires explicit opt-in by setting `auto_detect_document_type=True` in your configuration.

### All Optional Dependencies

To install Kreuzberg with all optional dependencies, you can use the `all` extra group:

```shell
pip install "kreuzberg[all]"
```

This is equivalent to:

```shell
pip install "kreuzberg[api,chunking,cli,crypto,document-classification,easyocr,entity-extraction,gmft,langdetect,paddleocr,additional-extensions]"
```

## Development Setup

For development and testing, additional system dependencies and language packs are required:

### Required System Dependencies

```shell
# Ubuntu/Debian
sudo apt-get install tesseract-ocr tesseract-ocr-deu pandoc

# macOS
brew install tesseract tesseract-lang pandoc

# Windows
choco install -y tesseract pandoc
# Then install German language pack as shown above
```

### Testing Requirements

The test suite includes OCR tests with German language documents that require the `deu` (German) language pack for Tesseract. Ensure the German language pack is installed as described in the Language Support section above.

To verify your development setup:

```shell
# Verify Tesseract has German support
tesseract --list-langs | grep deu

# Run the test suite
uv sync --all-extras --all-groups
uv run pytest
```
