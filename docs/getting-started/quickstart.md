# Quick Start

Get up and running with Kreuzberg in minutes.

## Basic Extraction

Extract text from any supported document format:

=== "Python"

    --8<-- "snippets/python/extract_file_sync.md"

=== "TypeScript"

    --8<-- "snippets/typescript/extract_file_sync.md"

=== "Rust"

    --8<-- "snippets/rust/extract_file_sync.md"

=== "Ruby"

    --8<-- "snippets/ruby/extract_file_sync.md"

=== "Java"

    --8<-- "snippets/java/extract_file_sync.md"

=== "Go"

    --8<-- "snippets/go/extract_file_sync.md"

=== "C#"

    --8<-- "snippets/csharp/extract_file_sync.md"

=== "CLI"

    --8<-- "snippets/cli/extract_basic.md"

## Async Extraction

For better performance with I/O-bound operations:

=== "Python"

    --8<-- "snippets/python/extract_file_async.md"

=== "TypeScript"

    --8<-- "snippets/typescript/extract_file_async.md"

=== "Rust"

    --8<-- "snippets/rust/extract_file_async.md"

=== "Ruby"

    --8<-- "snippets/ruby/extract_file_async.md"

=== "Java"

    --8<-- "snippets/java/extract_file_async.md"

=== "Go"

    --8<-- "snippets/go/extract_file_async.md"

=== "C#"

    --8<-- "snippets/csharp/extract_file_async.md"

=== "CLI"

    !!! note "Not Applicable"
        Async extraction is an API-level feature. The CLI operates synchronously.
        Use language-specific bindings (Python, TypeScript, Rust) for async operations.

## OCR Extraction

Extract text from images and scanned documents:

=== "Python"

    --8<-- "snippets/python/ocr_extraction.md"

=== "TypeScript"

    --8<-- "snippets/typescript/ocr_extraction.md"

=== "Rust"

    --8<-- "snippets/rust/ocr_extraction.md"

=== "Ruby"

    --8<-- "snippets/ruby/ocr_extraction.md"

=== "Java"

    --8<-- "snippets/java/ocr_extraction.md"

=== "Go"

    --8<-- "snippets/go/ocr_extraction.md"

=== "C#"

    --8<-- "snippets/csharp/ocr_extraction.md"

=== "CLI"

    --8<-- "snippets/cli/ocr_basic.md"

## Batch Processing

Process multiple files concurrently:

=== "Python"

    --8<-- "snippets/python/batch_extract_files_sync.md"

=== "TypeScript"

    --8<-- "snippets/typescript/batch_extract_files_sync.md"

=== "Rust"

    --8<-- "snippets/rust/batch_extract_files_sync.md"

=== "Ruby"

    --8<-- "snippets/ruby/batch_extract_files_sync.md"

=== "Java"

    --8<-- "snippets/java/batch_extract_files_sync.md"

=== "Go"

    --8<-- "snippets/go/batch_extract_files_sync.md"

=== "C#"

    --8<-- "snippets/csharp/batch_extract_files_sync.md"

=== "CLI"

    --8<-- "snippets/cli/batch_basic.md"

## Extract from Bytes

When you already have file content in memory:

=== "Python"

    --8<-- "snippets/python/extract_bytes_sync.md"

=== "TypeScript"

    --8<-- "snippets/typescript/extract_bytes_sync.md"

=== "Rust"

    --8<-- "snippets/rust/extract_bytes_sync.md"

=== "Ruby"

    --8<-- "snippets/ruby/extract_bytes_sync.md"

=== "Java"

    --8<-- "snippets/java/extract_bytes_sync.md"

=== "Go"

    --8<-- "snippets/go/extract_bytes_sync.md"

=== "C#"

    --8<-- "snippets/csharp/extract_bytes_sync.md"

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

=== "Python"

    --8<-- "snippets/python/advanced_config.md"

=== "TypeScript"

    --8<-- "snippets/typescript/advanced_config.md"

=== "Rust"

    --8<-- "snippets/rust/advanced_config.md"

=== "Ruby"

    --8<-- "snippets/ruby/advanced_config.md"

=== "Java"

    --8<-- "snippets/java/advanced_config.md"

=== "Go"

    --8<-- "snippets/go/advanced_config.md"

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

    ```toml
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

=== "Python"

    --8<-- "snippets/python/metadata.md"

=== "TypeScript"

    --8<-- "snippets/typescript/metadata.md"

=== "Rust"

    --8<-- "snippets/rust/metadata.md"

=== "Ruby"

    --8<-- "snippets/ruby/metadata.md"

=== "Java"

    --8<-- "snippets/java/metadata.md"

=== "Go"

    --8<-- "snippets/go/metadata.md"

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

=== "Python"

    --8<-- "snippets/python/tables.md"

=== "TypeScript"

    --8<-- "snippets/typescript/tables.md"

=== "Rust"

    --8<-- "snippets/rust/tables.md"

=== "Ruby"

    --8<-- "snippets/ruby/tables.md"

=== "Java"

    --8<-- "snippets/java/tables.md"

=== "Go"

    --8<-- "snippets/go/tables.md"

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

=== "Python"

    --8<-- "snippets/python/error_handling.md"

=== "TypeScript"

    --8<-- "snippets/typescript/error_handling.md"

=== "Rust"

    --8<-- "snippets/rust/error_handling.md"

=== "Ruby"

    --8<-- "snippets/ruby/error_handling.md"

=== "Java"

    --8<-- "snippets/java/error_handling.md"

=== "Go"

    --8<-- "snippets/go/error_handling.md"

## Next Steps

- [Contributing](../contributing.md) - Learn how to contribute to Kreuzberg
