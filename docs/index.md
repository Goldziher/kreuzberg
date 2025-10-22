# Kreuzberg

A high-performance document intelligence framework with a Rust core and bindings for Python, TypeScript, and CLI.

**Version 4 documentation is under construction.**

For v3 documentation, please visit the [main branch](https://github.com/Goldziher/kreuzberg).

## Features

- **Fast Rust Core**: All extraction logic implemented in Rust for maximum performance
- **Multi-Language Support**: Python, TypeScript/Node.js, and Rust/CLI bindings
- **Rich Format Support**: PDFs, images, office documents, emails, archives, and more
- **OCR Integration**: Tesseract, EasyOCR, PaddleOCR support
- **Extensible Plugin System**: Custom extractors, OCR backends, and post-processors
- **Table Extraction**: Detect and extract tables from documents
- **Keyword Extraction**: YAKE and RAKE algorithms for keyword detection
- **Language Detection**: Automatic language detection for extracted text
- **Content Chunking**: Smart chunking for LLM processing
- **Token Reduction**: Reduce token count while preserving meaning

## Quick Start

=== "Python"

    ```bash
    pip install kreuzberg
    ```

    ```python
    from kreuzberg import extract_file_sync

    result = extract_file_sync("document.pdf")
    print(result.content)
    ```

=== "TypeScript"

    ```bash
    npm install @goldziher/kreuzberg
    ```

    ```typescript
    import { extractFileSync } from '@goldziher/kreuzberg';

    const result = extractFileSync('document.pdf');
    console.log(result.content);
    ```

=== "CLI"

    ```bash
    brew install kreuzberg
    # or
    cargo install kreuzberg-cli
    ```

    ```bash
    kreuzberg extract document.pdf
    ```

## Links

- [GitHub Repository](https://github.com/Goldziher/kreuzberg)
- [PyPI](https://pypi.org/project/kreuzberg/)
- [npm](https://www.npmjs.com/package/@goldziher/kreuzberg)
- [Discord Community](https://discord.gg/pXxagNK2zN)
