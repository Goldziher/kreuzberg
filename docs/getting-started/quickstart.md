# Quick Start

Get up and running with Kreuzberg in minutes.

!!! info "Choosing Your TypeScript Package"

    Kreuzberg provides **two TypeScript packages** for different runtimes:

    - **`@kreuzberg/node`** – Use for Node.js servers and CLI tools (native performance, 100% speed)
    - **`@kreuzberg/wasm`** – Use for browsers, Cloudflare Workers, Deno, Bun, and serverless (60-80% speed, cross-platform)

    The examples below show both. Pick the one matching your runtime. See [Platform Overview](../index.md#choosing-between-typescript-packages) for detailed guidance.

## Basic Extraction

Extract text from any supported document format:

=== "C#"

    --8<-- "snippets/csharp/extract_file_sync.md"

=== "Go"

    --8<-- "snippets/go/api/extract_file_sync.md"

=== "Java"

    --8<-- "snippets/java/api/extract_file_sync.md"

=== "Python"

    --8<-- "snippets/python/api/extract_file_sync.md"

=== "Ruby"

    --8<-- "snippets/ruby/api/extract_file_sync.md"

=== "Rust"

    --8<-- "snippets/rust/api/extract_file_sync.md"

=== "TypeScript"

    --8<-- "snippets/typescript/getting-started/extract_file_sync.md"

=== "WASM"

    --8<-- "snippets/wasm/getting-started/extract_file_sync.md"

=== "CLI"

    --8<-- "snippets/cli/extract_basic.md"

## Async Extraction

For better performance with I/O-bound operations:

=== "C#"

    --8<-- "snippets/csharp/extract_file_async.md"

=== "Go"

    --8<-- "snippets/go/api/extract_file_async.md"

=== "Java"

    --8<-- "snippets/java/api/extract_file_async.md"

=== "Python"

    --8<-- "snippets/python/api/extract_file_async.md"

=== "Ruby"

    --8<-- "snippets/ruby/api/extract_file_async.md"

=== "Rust"

    --8<-- "snippets/rust/api/extract_file_async.md"

=== "TypeScript"

    --8<-- "snippets/typescript/getting-started/extract_file_async.md"

=== "WASM"

    --8<-- "snippets/wasm/getting-started/extract_file_async.md"

=== "CLI"

    !!! note "Not Applicable"
        Async extraction is an API-level feature. The CLI operates synchronously.
        Use language-specific bindings (Python, TypeScript, Rust, WASM) for async operations.

## OCR Extraction

Extract text from images and scanned documents:

=== "C#"

    --8<-- "snippets/csharp/ocr_extraction.md"

=== "Go"

    --8<-- "snippets/go/ocr/ocr_extraction.md"

=== "Java"

    --8<-- "snippets/java/ocr/ocr_extraction.md"

=== "Python"

    --8<-- "snippets/python/ocr/ocr_extraction.md"

=== "Ruby"

    --8<-- "snippets/ruby/ocr/ocr_extraction.md"

=== "Rust"

    --8<-- "snippets/rust/ocr/ocr_extraction.md"

=== "TypeScript"

    --8<-- "snippets/typescript/ocr/ocr_extraction.md"

=== "WASM"

    --8<-- "snippets/wasm/ocr/ocr_extraction.md"

=== "CLI"

    --8<-- "snippets/cli/ocr_basic.md"

## Batch Processing

Process multiple files concurrently:

=== "C#"

    --8<-- "snippets/csharp/batch_extract_files_sync.md"

=== "Go"

    --8<-- "snippets/go/api/batch_extract_files_sync.md"

=== "Java"

    --8<-- "snippets/java/api/batch_extract_files_sync.md"

=== "Python"

    --8<-- "snippets/python/api/batch_extract_files_sync.md"

=== "Ruby"

    --8<-- "snippets/ruby/api/batch_extract_files_sync.md"

=== "Rust"

    --8<-- "snippets/rust/api/batch_extract_files_sync.md"

=== "TypeScript"

    --8<-- "snippets/typescript/getting-started/batch_extract_files_sync.md"

=== "WASM"

    --8<-- "snippets/wasm/getting-started/batch_extract_files_sync.md"

=== "CLI"

    --8<-- "snippets/cli/batch_basic.md"

## Extract from Bytes

When you already have file content in memory:

=== "C#"

    --8<-- "snippets/csharp/extract_bytes_sync.md"

=== "Go"

    --8<-- "snippets/go/api/extract_bytes_sync.md"

=== "Java"

    --8<-- "snippets/java/api/extract_bytes_sync.md"

=== "Python"

    --8<-- "snippets/python/api/extract_bytes_sync.md"

=== "Ruby"

    --8<-- "snippets/ruby/api/extract_bytes_sync.md"

=== "Rust"

    --8<-- "snippets/rust/api/extract_bytes_sync.md"

=== "TypeScript"

    --8<-- "snippets/typescript/getting-started/extract_bytes_sync.md"

=== "WASM"

    --8<-- "snippets/wasm/getting-started/extract_bytes_sync.md"

=== "CLI"

    !!! note "Not Applicable"
        The CLI operates on files from disk. For in-memory data processing, use language-specific bindings.

        However, you can use CLI with pipes and temporary files:

        ```bash
        # Create temporary file from stdin and extract
        cat data.pdf | kreuzberg extract /dev/stdin

        # Or process piped content
        curl https://example.com/document.pdf | \
          kreuzberg extract /dev/stdin
        ```

## Advanced Configuration

Customize extraction behavior:

=== "C#"

    --8<-- "snippets/csharp/advanced_config.md"

=== "Go"

    --8<-- "snippets/go/config/advanced_config.md"

=== "Java"

    --8<-- "snippets/java/config/advanced_config.md"

=== "Python"

    --8<-- "snippets/python/advanced/advanced_config.md"

=== "Ruby"

    --8<-- "snippets/ruby/config/advanced_config.md"

=== "Rust"

    --8<-- "snippets/rust/advanced/advanced_config.md"

=== "TypeScript"

    --8<-- "snippets/typescript/config/advanced_config.md"

=== "WASM"

    --8<-- "snippets/wasm/config/advanced_config.md"

=== "CLI"

    Configure extraction behavior via command-line flags or config files:

    ```bash
    # Using command-line flags
    kreuzberg extract document.pdf \
      --ocr \
      --chunk --chunk-size 1000 --chunk-overlap 100 \
      --detect-language \
      --quality

    # Using config file
    kreuzberg extract document.pdf --config kreuzberg.toml
    ```

    **kreuzberg.toml:**

    ```toml title="TOML"
    [ocr]
    backend = "tesseract"
    language = "eng"

    [chunking]
    max_chunk_size = 1000
    overlap = 100

    [language_detection]
    enabled = true
    detect_multiple = true

    enable_quality_processing = true
    use_cache = true
    ```

    **kreuzberg.yaml:**

    ```yaml
    ocr:
      backend: tesseract
      language: eng

    chunking:
      max_chunk_size: 1000
      overlap: 100

    language_detection:
      enabled: true
      detect_multiple: true

    enable_quality_processing: true
    use_cache: true
    ```

## Working with Metadata

Access format-specific metadata from extracted documents:

=== "C#"

    --8<-- "snippets/csharp/metadata.md"

=== "Go"

    --8<-- "snippets/go/metadata/metadata.md"

=== "Java"

    --8<-- "snippets/java/metadata/metadata.md"

=== "Python"

    --8<-- "snippets/python/metadata/metadata.md"

=== "Ruby"

    --8<-- "snippets/ruby/metadata/metadata.md"

=== "Rust"

    --8<-- "snippets/rust/metadata/metadata.md"

=== "TypeScript"

    --8<-- "snippets/typescript/metadata/metadata.md"

=== "WASM"

    --8<-- "snippets/wasm/metadata/metadata.md"

=== "CLI"

    Extract and parse metadata using JSON output:

    ```bash
    # Extract with metadata
    kreuzberg extract document.pdf --metadata --format json --pretty

    # Save to file and parse metadata
    kreuzberg extract document.pdf --metadata --format json > result.json

    # Extract PDF metadata
    cat result.json | jq '.metadata.pdf'

    # Extract HTML metadata
    kreuzberg extract page.html --metadata --format json | jq '.metadata.html'

    # Get specific fields
    kreuzberg extract document.pdf --metadata --format json | \
      jq '.metadata | {page_count, author, title}'

    # Process multiple files
    kreuzberg batch documents/*.pdf --metadata --format json > all_metadata.json
    ```

    **JSON Output Structure:**

    ```json
    {
      "content": "Extracted text...",
      "metadata": {
        "mime_type": "application/pdf",
        "pdf": {
          "page_count": 10,
          "author": "John Doe",
          "title": "Document Title"
        }
      }
    }
    ```

Kreuzberg extracts format-specific metadata for:
- **PDF**: page count, title, author, subject, keywords, dates
- **HTML**: 21 fields including SEO meta tags, Open Graph, Twitter Card
- **Excel**: sheet count, sheet names
- **Email**: from, to, CC, BCC, message ID, attachments
- **PowerPoint**: title, author, description, fonts
- **Images**: dimensions, format, EXIF data
- **Archives**: format, file count, file list, sizes
- **XML**: element count, unique elements
- **Text/Markdown**: word count, line count, headers, links

See [Types Reference](../reference/types.md) for complete metadata reference.

## Working with Tables

Extract and process tables from documents:

=== "C#"

    --8<-- "snippets/csharp/tables.md"

=== "Go"

    --8<-- "snippets/go/metadata/tables.md"

=== "Java"

    --8<-- "snippets/java/metadata/tables.md"

=== "Python"

    --8<-- "snippets/python/utils/tables.md"

=== "Ruby"

    --8<-- "snippets/ruby/metadata/tables.md"

=== "Rust"

    --8<-- "snippets/rust/metadata/tables.md"

=== "TypeScript"

    --8<-- "snippets/typescript/api/tables.md"

=== "WASM"

    --8<-- "snippets/wasm/api/tables.md"

=== "CLI"

    Extract and process tables from documents:

    ```bash
    # Extract tables
    kreuzberg extract document.pdf --tables --format json --pretty

    # Save tables to JSON
    kreuzberg extract spreadsheet.xlsx --tables --format json > tables.json

    # Extract and parse table markdown
    kreuzberg extract document.pdf --tables --format json | \
      jq '.tables[] | .markdown'

    # Get table cells
    kreuzberg extract document.pdf --tables --format json | \
      jq '.tables[] | .cells'

    # Batch extract tables from multiple files
    kreuzberg batch documents/**/*.pdf --tables --format json > all_tables.json
    ```

    **JSON Table Structure:**

    ```json
    {
      "content": "...",
      "tables": [
        {
          "cells": [
            ["Name", "Age", "City"],
            ["Alice", "30", "New York"],
            ["Bob", "25", "Los Angeles"]
          ],
          "markdown": "| Name | Age | City |\n|------|-----|--------|\n| Alice | 30 | New York |\n| Bob | 25 | Los Angeles |"
        }
      ]
    }
    ```

## Error Handling

Handle extraction errors gracefully:

=== "C#"

    --8<-- "snippets/csharp/error_handling.md"

=== "Go"

    --8<-- "snippets/go/api/error_handling.md"

=== "Java"

    --8<-- "snippets/java/api/error_handling.md"

=== "Python"

    --8<-- "snippets/python/utils/error_handling.md"

=== "Ruby"

    --8<-- "snippets/ruby/api/error_handling.md"

=== "Rust"

    --8<-- "snippets/rust/api/error_handling.md"

=== "TypeScript"

    --8<-- "snippets/typescript/api/error_handling.md"

=== "WASM"

    --8<-- "snippets/wasm/api/error_handling.md"

## Next Steps

- [Contributing](../contributing.md) - Learn how to contribute to Kreuzberg
