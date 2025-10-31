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

=== "Rust"

    ```rust
    use kreuzberg::extract_file_sync;

    fn main() -> kreuzberg::Result<()> {
        // Extract from a file
        let result = extract_file_sync("document.pdf", None, &Default::default())?;

        println!("{}", result.content);  // Extracted text
        println!("{:?}", result.metadata);  // Document metadata
        println!("{:?}", result.tables);    // Extracted tables
        Ok(())
    }
    ```

=== "Ruby"

    ```ruby
    require 'kreuzberg'

    # Extract from a file
    result = Kreuzberg.extract_file_sync('document.pdf')

    puts result.content  # Extracted text
    puts result.metadata  # Document metadata
    puts result.tables    # Extracted tables
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

=== "Rust"

    ```rust
    use kreuzberg::extract_file;

    #[tokio::main]
    async fn main() -> kreuzberg::Result<()> {
        let result = extract_file("document.pdf", None, &Default::default()).await?;
        println!("{}", result.content);
        Ok(())
    }
    ```

=== "Ruby"

    ```ruby
    require 'kreuzberg'

    # Ruby doesn't have native async/await
    # This uses a blocking Tokio runtime internally
    result = Kreuzberg.extract_file('document.pdf')
    puts result.content
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

=== "Rust"

    ```rust
    use kreuzberg::{extract_file_sync, ExtractionConfig, OcrConfig};

    fn main() -> kreuzberg::Result<()> {
        let config = ExtractionConfig {
            ocr: Some(OcrConfig {
                backend: "tesseract".to_string(),
                language: Some("eng".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        };

        let result = extract_file_sync("scanned.pdf", None, &config)?;
        println!("{}", result.content);
        Ok(())
    }
    ```

=== "Ruby"

    ```ruby
    require 'kreuzberg'

    config = Kreuzberg::Config::Extraction.new(
      ocr: Kreuzberg::Config::OCR.new(
        backend: 'tesseract',
        language: 'eng'
      )
    )

    result = Kreuzberg.extract_file_sync('scanned.pdf', config: config)
    puts result.content
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

=== "Rust"

    ```rust
    use kreuzberg::{batch_extract_file_sync, ExtractionConfig};

    fn main() -> kreuzberg::Result<()> {
        let files = vec!["doc1.pdf", "doc2.docx", "doc3.pptx"];
        let results = batch_extract_file_sync(&files, None, &ExtractionConfig::default())?;

        for result in results {
            println!("Content length: {}", result.content.len());
        }
        Ok(())
    }
    ```

=== "Ruby"

    ```ruby
    require 'kreuzberg'

    files = ['doc1.pdf', 'doc2.docx', 'doc3.pptx']
    results = Kreuzberg.batch_extract_files_sync(files)

    results.each do |result|
      puts "Content length: #{result.content.length}"
    end
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

=== "Rust"

    ```rust
    use kreuzberg::{extract_bytes_sync, ExtractionConfig};
    use std::fs;

    fn main() -> kreuzberg::Result<()> {
        let data = fs::read("document.pdf")?;

        let result = extract_bytes_sync(
            &data,
            "application/pdf",
            &ExtractionConfig::default()
        )?;
        println!("{}", result.content);
        Ok(())
    }
    ```

=== "Ruby"

    ```ruby
    require 'kreuzberg'

    data = File.binread('document.pdf')

    result = Kreuzberg.extract_bytes_sync(
      data,
      'application/pdf'
    )
    puts result.content
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

=== "Rust"

    ```rust
    use kreuzberg::{
        extract_file_sync,
        ExtractionConfig,
        OcrConfig,
        ChunkingConfig,
        LanguageDetectionConfig
    };

    fn main() -> kreuzberg::Result<()> {
        let config = ExtractionConfig {
            // Enable OCR
            ocr: Some(OcrConfig {
                backend: "tesseract".to_string(),
                language: Some("eng+deu".to_string()),  // Multiple languages
                ..Default::default()
            }),

            // Enable chunking for LLM processing
            chunking: Some(ChunkingConfig {
                max_chunk_size: 1000,
                overlap: 100,
            }),

            // Enable language detection
            language_detection: Some(LanguageDetectionConfig {
                enabled: true,
                detect_multiple: true,
                ..Default::default()
            }),

            // Enable caching
            use_cache: true,

            // Enable quality processing
            enable_quality_processing: true,

            ..Default::default()
        };

        let result = extract_file_sync("document.pdf", None, &config)?;

        // Access chunks
        if let Some(chunks) = result.chunks {
            for chunk in chunks {
                println!("Chunk: {}...", &chunk[..100.min(chunk.len())]);
            }
        }

        // Access detected languages
        if let Some(languages) = result.detected_languages {
            println!("Languages: {:?}", languages);
        }
        Ok(())
    }
    ```

=== "Ruby"

    ```ruby
    require 'kreuzberg'

    config = Kreuzberg::Config::Extraction.new(
      # Enable OCR
      ocr: Kreuzberg::Config::OCR.new(
        backend: 'tesseract',
        language: 'eng+deu'  # Multiple languages
      ),

      # Enable chunking for LLM processing
      chunking: Kreuzberg::Config::Chunking.new(
        max_chars: 1000,
        max_overlap: 100
      ),

      # Enable language detection
      language_detection: Kreuzberg::Config::LanguageDetection.new,

      # Enable caching
      use_cache: true,

      # Enable quality processing
      enable_quality_processing: true
    )

    result = Kreuzberg.extract_file_sync('document.pdf', config: config)

    # Access chunks
    if result.chunks
      result.chunks.each do |chunk|
        puts "Chunk: #{chunk[0..100]}..."
      end
    end

    # Access detected languages
    if result.detected_languages
      puts "Languages: #{result.detected_languages}"
    end
    ```

## Working with Metadata

Access format-specific metadata from extracted documents:

=== "Python"

    ```python
    from kreuzberg import extract_file_sync, ExtractionConfig

    result = extract_file_sync("document.pdf", config=ExtractionConfig())

    # Access PDF metadata
    if result.metadata.get("pdf"):
        pdf_meta = result.metadata["pdf"]
        print(f"Pages: {pdf_meta.get('page_count')}")
        print(f"Author: {pdf_meta.get('author')}")
        print(f"Title: {pdf_meta.get('title')}")

    # Access HTML metadata
    result = extract_file_sync("page.html", config=ExtractionConfig())
    if result.metadata.get("html"):
        html_meta = result.metadata["html"]
        print(f"Title: {html_meta.get('title')}")
        print(f"Description: {html_meta.get('description')}")
        print(f"Open Graph Image: {html_meta.get('og_image')}")
    ```

=== "TypeScript"

    ```typescript
    import { extractFileSync, ExtractionConfig } from '@goldziher/kreuzberg';

    const result = extractFileSync('document.pdf', null, new ExtractionConfig());

    // Access PDF metadata
    if (result.metadata.pdf) {
        console.log(`Pages: ${result.metadata.pdf.pageCount}`);
        console.log(`Author: ${result.metadata.pdf.author}`);
        console.log(`Title: ${result.metadata.pdf.title}`);
    }

    // Access HTML metadata
    const htmlResult = extractFileSync('page.html', null, new ExtractionConfig());
    if (htmlResult.metadata.html) {
        console.log(`Title: ${htmlResult.metadata.html.title}`);
        console.log(`Description: ${htmlResult.metadata.html.description}`);
        console.log(`Open Graph Image: ${htmlResult.metadata.html.ogImage}`);
    }
    ```

=== "Rust"

    ```rust
    use kreuzberg::{extract_file_sync, ExtractionConfig};

    fn main() -> kreuzberg::Result<()> {
        let result = extract_file_sync("document.pdf", None, &ExtractionConfig::default())?;

        // Access PDF metadata
        if let Some(pdf_meta) = result.metadata.pdf {
            if let Some(pages) = pdf_meta.page_count {
                println!("Pages: {}", pages);
            }
            if let Some(author) = pdf_meta.author {
                println!("Author: {}", author);
            }
            if let Some(title) = pdf_meta.title {
                println!("Title: {}", title);
            }
        }

        // Access HTML metadata
        let html_result = extract_file_sync("page.html", None, &ExtractionConfig::default())?;
        if let Some(html_meta) = html_result.metadata.html {
            if let Some(title) = html_meta.title {
                println!("Title: {}", title);
            }
            if let Some(desc) = html_meta.description {
                println!("Description: {}", desc);
            }
            if let Some(og_img) = html_meta.og_image {
                println!("Open Graph Image: {}", og_img);
            }
        }
        Ok(())
    }
    ```

=== "Ruby"

    ```ruby
    require 'kreuzberg'

    result = Kreuzberg.extract_file_sync('document.pdf')

    # Access PDF metadata
    if result.metadata['pdf']
      pdf_meta = result.metadata['pdf']
      puts "Pages: #{pdf_meta['page_count']}"
      puts "Author: #{pdf_meta['author']}"
      puts "Title: #{pdf_meta['title']}"
    end

    # Access HTML metadata
    html_result = Kreuzberg.extract_file_sync('page.html')
    if html_result.metadata['html']
      html_meta = html_result.metadata['html']
      puts "Title: #{html_meta['title']}"
      puts "Description: #{html_meta['description']}"
      puts "Open Graph Image: #{html_meta['og_image']}"
    end
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

See [Extractors Documentation](../concepts/extractors.md) for complete metadata reference.

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

=== "Rust"

    ```rust
    use kreuzberg::{extract_file_sync, ExtractionConfig};

    fn main() -> kreuzberg::Result<()> {
        let result = extract_file_sync("document.pdf", None, &ExtractionConfig::default())?;

        // Iterate over tables
        for table in &result.tables {
            println!("Table with {} rows", table.cells.len());
            println!("{}", table.markdown);  // Markdown representation

            // Access cells
            for row in &table.cells {
                println!("{:?}", row);
            }
        }
        Ok(())
    }
    ```

=== "Ruby"

    ```ruby
    require 'kreuzberg'

    result = Kreuzberg.extract_file_sync('document.pdf')

    # Iterate over tables
    result.tables.each do |table|
      puts "Table with #{table['cells'].length} rows"
      puts table['markdown']  # Markdown representation

      # Access cells
      table['cells'].each do |row|
        puts row
      end
    end
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

=== "Rust"

    ```rust
    use kreuzberg::{extract_file_sync, ExtractionConfig, KreuzbergError};

    fn main() {
        let result = extract_file_sync("document.pdf", None, &ExtractionConfig::default());

        match result {
            Ok(extraction) => {
                println!("{}", extraction.content);
            }
            Err(KreuzbergError::Validation(msg)) => {
                eprintln!("Invalid configuration: {}", msg);
            }
            Err(KreuzbergError::Parsing(msg)) => {
                eprintln!("Failed to parse document: {}", msg);
            }
            Err(KreuzbergError::Ocr(msg)) => {
                eprintln!("OCR processing failed: {}", msg);
            }
            Err(e) => {
                eprintln!("Extraction error: {}", e);
            }
        }
    }
    ```

=== "Ruby"

    ```ruby
    require 'kreuzberg'

    begin
      result = Kreuzberg.extract_file_sync('document.pdf')
      puts result.content
    rescue StandardError => e
      puts "Extraction error: #{e.message}"
    end
    ```

## Next Steps

- [Contributing](../contributing.md) - Learn how to contribute to Kreuzberg
