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

## Optional Features

### OCR

OCR is an optional feature for extracting text from images and non-searchable PDFs. Kreuzberg supports multiple OCR backends. To understand the differences between these backends, please read the [OCR Backends documentation](../user-guide/ocr-backends.md).

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
    - **macOS**: `brew install tesseract-lang`
    - **Windows**: See the [Tesseract documentation](https://tesseract-ocr.github.io/tessdoc/Installation.html#windows)

    For more details on language installation and configuration, refer to the [Tesseract documentation](https://tesseract-ocr.github.io/tessdoc/Installation.html).

#### EasyOCR

EasyOCR is a Python-based OCR backend with wide language support and strong performance.

```shell
pip install "kreuzberg[easyocr]"
```

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

### Chunking

Chunking is an optional feature - useful for RAG applications among others. Kreuzberg uses the excellent `semantic-text-splitter` package for chunking. To install Kreuzberg with chunking support, you can use:

```shell
pip install "kreuzberg[chunking]"
```

### Table Extraction

Kreuzberg offers multiple approaches for extracting tables from documents:

#### Vision-Based Table Extraction (Recommended)

Uses computer vision models for high-accuracy table detection and structure recognition. Best for complex tables and diverse document types.

```shell
pip install "kreuzberg[gmft]"
```

**Features:**

- Complex table layouts with spanning cells and multi-level headers
- Works on any document type (PDFs, images, presentations)
- GPU acceleration support
- ~1GB model download on first use

#### OCR-Based Table Extraction (Lightweight)

Uses Tesseract OCR analysis to detect simple table structures. Included with the base installation.

```shell
pip install kreuzberg  # Already included
```

**Features:**

- Fast processing with minimal resource usage
- No additional dependencies beyond Tesseract
- Good for simple, well-formatted tables
- Works with scanned documents

See the [Table Extraction Guide](../user-guide/table-extraction.md) for detailed comparison and usage instructions.

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
