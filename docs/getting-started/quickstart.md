# Quick Start

Get up and running with Kreuzberg in minutes.

## Basic Extraction

Extract text from any supported document format:

=== "Python"

    ```python
    from kreuzberg import extract_file_sync

    # Extract from a file
    result = extract_file_sync("document.pdf")

    print(result.content)  # Extracted text
    print(result.metadata)  # Document metadata
    print(result.tables)    # Extracted tables
    ```

=== "TypeScript"

    ```typescript
    import { extractFileSync } from '@goldziher/kreuzberg';

    // Extract from a file
    const result = extractFileSync('document.pdf');

    console.log(result.content);  // Extracted text
    console.log(result.metadata);  // Document metadata
    console.log(result.tables);    // Extracted tables
    ```

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

    ```python
    import asyncio
    from kreuzberg import extract_file

    async def main():
        result = await extract_file("document.pdf")
        print(result.content)

    asyncio.run(main())
    ```

=== "TypeScript"

    ```typescript
    import { extractFile } from '@goldziher/kreuzberg';

    async function main() {
        const result = await extractFile('document.pdf');
        console.log(result.content);
    }

    main();
    ```

## OCR Extraction

Extract text from images and scanned documents:

=== "Python"

    ```python
    from kreuzberg import extract_file_sync, ExtractionConfig, OcrConfig

    config = ExtractionConfig(
        ocr=OcrConfig(
            backend="tesseract",
            language="eng"
        )
    )

    result = extract_file_sync("scanned.pdf", config=config)
    print(result.content)
    ```

=== "TypeScript"

    ```typescript
    import { extractFileSync, ExtractionConfig, OcrConfig } from '@goldziher/kreuzberg';

    const config = new ExtractionConfig({
        ocr: new OcrConfig({
            backend: 'tesseract',
            language: 'eng'
        })
    });

    const result = extractFileSync('scanned.pdf', null, config);
    console.log(result.content);
    ```

=== "CLI"

    ```bash
    kreuzberg extract scanned.pdf --ocr --language eng
    ```

## Batch Processing

Process multiple files concurrently:

=== "Python"

    ```python
    from kreuzberg import batch_extract_files_sync, ExtractionConfig

    files = ["doc1.pdf", "doc2.docx", "doc3.pptx"]
    results = batch_extract_files_sync(files, config=ExtractionConfig())

    for result in results:
        print(f"Content length: {len(result.content)}")
    ```

=== "TypeScript"

    ```typescript
    import { batchExtractFilesSync, ExtractionConfig } from '@goldziher/kreuzberg';

    const files = ['doc1.pdf', 'doc2.docx', 'doc3.pptx'];
    const results = batchExtractFilesSync(files, new ExtractionConfig());

    for (const result of results) {
        console.log(`Content length: ${result.content.length}`);
    }
    ```

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

    ```python
    from kreuzberg import extract_bytes_sync, ExtractionConfig

    with open("document.pdf", "rb") as f:
        data = f.read()

    result = extract_bytes_sync(
        data,
        mime_type="application/pdf",
        config=ExtractionConfig()
    )
    print(result.content)
    ```

=== "TypeScript"

    ```typescript
    import { extractBytesSync, ExtractionConfig } from '@goldziher/kreuzberg';
    import { readFileSync } from 'fs';

    const data = readFileSync('document.pdf');

    const result = extractBytesSync(
        data,
        'application/pdf',
        new ExtractionConfig()
    );
    console.log(result.content);
    ```

## Advanced Configuration

Customize extraction behavior:

=== "Python"

    ```python
    from kreuzberg import (
        extract_file_sync,
        ExtractionConfig,
        OcrConfig,
        ChunkingConfig,
        TokenReductionConfig,
        LanguageDetectionConfig
    )

    config = ExtractionConfig(
        # Enable OCR
        ocr=OcrConfig(
            backend="tesseract",
            language="eng+deu"  # Multiple languages
        ),

        # Enable chunking for LLM processing
        chunking=ChunkingConfig(
            max_chunk_size=1000,
            overlap=100
        ),

        # Enable token reduction
        token_reduction=TokenReductionConfig(
            enabled=True,
            target_reduction=0.3  # Reduce by 30%
        ),

        # Enable language detection
        language_detection=LanguageDetectionConfig(
            enabled=True,
            detect_multiple=True
        ),

        # Enable caching
        use_cache=True,

        # Enable quality processing
        enable_quality_processing=True
    )

    result = extract_file_sync("document.pdf", config=config)

    # Access chunks
    for chunk in result.chunks:
        print(f"Chunk: {chunk.text[:100]}...")

    # Access detected languages
    if result.detected_languages:
        print(f"Languages: {result.detected_languages}")
    ```

=== "TypeScript"

    ```typescript
    import {
        extractFileSync,
        ExtractionConfig,
        OcrConfig,
        ChunkingConfig,
        TokenReductionConfig,
        LanguageDetectionConfig
    } from '@goldziher/kreuzberg';

    const config = new ExtractionConfig({
        // Enable OCR
        ocr: new OcrConfig({
            backend: 'tesseract',
            language: 'eng+deu'  // Multiple languages
        }),

        // Enable chunking for LLM processing
        chunking: new ChunkingConfig({
            maxChunkSize: 1000,
            overlap: 100
        }),

        // Enable token reduction
        tokenReduction: new TokenReductionConfig({
            enabled: true,
            targetReduction: 0.3  // Reduce by 30%
        }),

        // Enable language detection
        languageDetection: new LanguageDetectionConfig({
            enabled: true,
            detectMultiple: true
        }),

        // Enable caching
        useCache: true,

        // Enable quality processing
        enableQualityProcessing: true
    });

    const result = extractFileSync('document.pdf', null, config);

    // Access chunks
    for (const chunk of result.chunks) {
        console.log(`Chunk: ${chunk.text.substring(0, 100)}...`);
    }

    // Access detected languages
    if (result.detectedLanguages) {
        console.log(`Languages: ${result.detectedLanguages}`);
    }
    ```

## Working with Tables

Extract and process tables from documents:

=== "Python"

    ```python
    from kreuzberg import extract_file_sync, ExtractionConfig

    result = extract_file_sync("document.pdf", config=ExtractionConfig())

    # Iterate over tables
    for table in result.tables:
        print(f"Table with {len(table.cells)} rows")
        print(table.markdown)  # Markdown representation

        # Access cells
        for row in table.cells:
            print(row)
    ```

=== "TypeScript"

    ```typescript
    import { extractFileSync, ExtractionConfig } from '@goldziher/kreuzberg';

    const result = extractFileSync('document.pdf', null, new ExtractionConfig());

    // Iterate over tables
    for (const table of result.tables) {
        console.log(`Table with ${table.cells.length} rows`);
        console.log(table.markdown);  // Markdown representation

        // Access cells
        for (const row of table.cells) {
            console.log(row);
        }
    }
    ```

## Error Handling

Handle extraction errors gracefully:

=== "Python"

    ```python
    from kreuzberg import (
        extract_file_sync,
        ExtractionConfig,
        KreuzbergError,
        ValidationError,
        ParsingError,
        OCRError
    )

    try:
        result = extract_file_sync("document.pdf", config=ExtractionConfig())
        print(result.content)
    except ValidationError as e:
        print(f"Invalid configuration: {e}")
    except ParsingError as e:
        print(f"Failed to parse document: {e}")
    except OCRError as e:
        print(f"OCR processing failed: {e}")
    except KreuzbergError as e:
        print(f"Extraction error: {e}")
    ```

=== "TypeScript"

    ```typescript
    import {
        extractFileSync,
        ExtractionConfig,
        KreuzbergError
    } from '@goldziher/kreuzberg';

    try {
        const result = extractFileSync('document.pdf', null, new ExtractionConfig());
        console.log(result.content);
    } catch (error) {
        if (error instanceof KreuzbergError) {
            console.error(`Extraction error: ${error.message}`);
        } else {
            throw error;
        }
    }
    ```

## Next Steps

- [Contributing](../contributing.md) - Learn how to contribute to Kreuzberg
