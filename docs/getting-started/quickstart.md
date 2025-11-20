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
    } from 'kreuzberg';

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

=== "Java"

    ```java
    import dev.kreuzberg.Kreuzberg;
    import dev.kreuzberg.ExtractionResult;
    import dev.kreuzberg.KreuzbergException;
    import dev.kreuzberg.config.*;
    import java.io.IOException;

    public class Main {
        public static void main(String[] args) {
            try {
                ExtractionConfig config = ExtractionConfig.builder()
                    // Enable OCR
                    .ocr(OcrConfig.builder()
                        .backend("tesseract")
                        .language("eng+deu")  // Multiple languages
                        .build())

                    // Enable chunking for LLM processing
                    .chunking(ChunkingConfig.builder()
                        .maxChars(1000)
                        .maxOverlap(100)
                        .build())

                    // Enable token reduction
                    .tokenReduction(TokenReductionConfig.builder()
                        .mode("moderate")
                        .preserveImportantWords(true)
                        .build())

                    // Enable language detection
                    .languageDetection(LanguageDetectionConfig.builder()
                        .enabled(true)
                        .build())

                    // Enable caching
                    .useCache(true)

                    // Enable quality processing
                    .enableQualityProcessing(true)
                    .build();

                ExtractionResult result = Kreuzberg.extractFileSync("document.pdf", null, config);

                // Access detected languages
                if (!result.getDetectedLanguages().isEmpty()) {
                    System.out.println("Languages: " + result.getDetectedLanguages());
                }
            } catch (IOException | KreuzbergException e) {
                System.err.println("Extraction failed: " + e.getMessage());
            }
        }
    }
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
    import { extractFileSync, ExtractionConfig } from 'kreuzberg';

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

=== "Java"

    ```java
    import dev.kreuzberg.Kreuzberg;
    import dev.kreuzberg.ExtractionResult;
    import dev.kreuzberg.KreuzbergException;
    import java.io.IOException;
    import java.util.Map;

    public class Main {
        public static void main(String[] args) {
            try {
                ExtractionResult result = Kreuzberg.extractFileSync("document.pdf");

                // Access PDF metadata
                @SuppressWarnings("unchecked")
                Map<String, Object> pdfMeta = (Map<String, Object>) result.getMetadata().get("pdf");
                if (pdfMeta != null) {
                    System.out.println("Pages: " + pdfMeta.get("page_count"));
                    System.out.println("Author: " + pdfMeta.get("author"));
                    System.out.println("Title: " + pdfMeta.get("title"));
                }

                // Access HTML metadata
                ExtractionResult htmlResult = Kreuzberg.extractFileSync("page.html");
                @SuppressWarnings("unchecked")
                Map<String, Object> htmlMeta = (Map<String, Object>) htmlResult.getMetadata().get("html");
                if (htmlMeta != null) {
                    System.out.println("Title: " + htmlMeta.get("title"));
                    System.out.println("Description: " + htmlMeta.get("description"));
                    System.out.println("Open Graph Image: " + htmlMeta.get("og_image"));
                }
            } catch (IOException | KreuzbergException e) {
                System.err.println("Extraction failed: " + e.getMessage());
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
    import { extractFileSync, ExtractionConfig } from 'kreuzberg';

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

=== "Java"

    ```java
    import dev.kreuzberg.Kreuzberg;
    import dev.kreuzberg.ExtractionResult;
    import dev.kreuzberg.KreuzbergException;
    import dev.kreuzberg.Table;
    import java.io.IOException;
    import java.util.List;

    public class Main {
        public static void main(String[] args) {
            try {
                ExtractionResult result = Kreuzberg.extractFileSync("document.pdf");

                // Iterate over tables
                for (Table table : result.getTables()) {
                    System.out.println("Table with " + table.cells().size() + " rows");
                    System.out.println(table.markdown());  // Markdown representation

                    // Access cells
                    for (List<String> row : table.cells()) {
                        System.out.println(row);
                    }
                }
            } catch (IOException | KreuzbergException e) {
                System.err.println("Extraction failed: " + e.getMessage());
            }
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
    } from 'kreuzberg';

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

=== "Java"

    ```java
    import dev.kreuzberg.Kreuzberg;
    import dev.kreuzberg.ExtractionResult;
    import dev.kreuzberg.KreuzbergException;
    import dev.kreuzberg.ValidationException;
    import dev.kreuzberg.ParsingException;
    import dev.kreuzberg.OcrException;
    import java.io.IOException;

    public class Main {
        public static void main(String[] args) {
            try {
                ExtractionResult result = Kreuzberg.extractFileSync("document.pdf");
                System.out.println(result.getContent());
            } catch (ValidationException e) {
                System.err.println("Invalid configuration: " + e.getMessage());
            } catch (ParsingException e) {
                System.err.println("Failed to parse document: " + e.getMessage());
            } catch (OcrException e) {
                System.err.println("OCR processing failed: " + e.getMessage());
            } catch (IOException e) {
                System.err.println("File error: " + e.getMessage());
            } catch (KreuzbergException e) {
                System.err.println("Extraction error: " + e.getMessage());
            }
        }
    }
    ```

## Next Steps

- [Contributing](../contributing.md) - Learn how to contribute to Kreuzberg
