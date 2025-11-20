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

=== "CLI"

    ```bash
    # Extract to stdout
    kreuzberg extract document.pdf

    # Save to file
    kreuzberg extract document.pdf -o output.txt

    # Extract with metadata
    kreuzberg extract document.pdf --metadata
    ```

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

=== "CLI"

    ```bash
    kreuzberg extract scanned.pdf --ocr --language eng
    ```

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

=== "CLI"

    ```bash
    # Process multiple files
    kreuzberg extract doc1.pdf doc2.docx doc3.pptx

    # Use glob patterns
    kreuzberg extract documents/**/*.pdf
    ```

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
