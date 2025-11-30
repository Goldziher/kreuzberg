# Format Support

Kreuzberg supports 56 file formats across major categories, providing comprehensive document intelligence capabilities through native Rust extractors, Pandoc integration, and LibreOffice conversion.

## Overview

Kreuzberg v4 uses a high-performance Rust core with three extraction methods:

- **Native Rust Extractors**: Fast, memory-efficient extractors for common formats
- **Pandoc Integration**: Support for 30+ academic and publishing formats
- **LibreOffice Conversion**: Legacy Microsoft Office format support (`.doc`, `.ppt`)

All formats support async/await and batch processing. Image formats and PDFs support optional OCR when configured.

## Format Support Matrix

### Office Documents

| Format | Extensions | MIME Type | Extraction Method | OCR Support | Special Features |
|--------|-----------|-----------|-------------------|-------------|------------------|
| PDF | `.pdf` | `application/pdf` | Native Rust (pdfium-render) | Yes | Metadata extraction, image extraction, text layer detection |
| Excel | `.xlsx`, `.xlsm`, `.xlsb`, `.xls`, `.xlam`, `.xla`, `.ods` | Various Excel MIME types | Native Rust (calamine) | No | Multi-sheet support, formula preservation |
| PowerPoint | `.pptx`, `.pptm`, `.ppsx` | `application/vnd.openxmlformats-officedocument.presentationml.presentation` | Native Rust (roxmltree) | Yes (for embedded images) | Slide extraction, image OCR, table detection |
| Word (Modern) | `.docx` | `application/vnd.openxmlformats-officedocument.wordprocessingml.document` | Pandoc | No | Preserves formatting, extracts metadata |
| Word (Legacy) | `.doc` | `application/msword` | LibreOffice + Pandoc | No | Converts to DOCX then extracts |
| PowerPoint (Legacy) | `.ppt` | `application/vnd.ms-powerpoint` | LibreOffice + Pandoc | No | Converts to PPTX then extracts |
| OpenDocument Text | `.odt` | `application/vnd.oasis.opendocument.text` | Pandoc | No | Full OpenDocument support |
| OpenDocument Spreadsheet | `.ods` | `application/vnd.oasis.opendocument.spreadsheet` | Native Rust (calamine) | No | Multi-sheet support |

### Text & Markup

| Format | Extensions | MIME Type | Extraction Method | OCR Support | Special Features |
|--------|-----------|-----------|-------------------|-------------|------------------|
| Plain Text | `.txt` | `text/plain` | Native Rust (streaming) | No | Line/word/character counting, memory-efficient streaming |
| Markdown | `.md`, `.markdown` | `text/markdown`, `text/x-markdown` | Native Rust (streaming) | No | Header extraction, link detection, code block detection |
| HTML | `.html`, `.htm` | `text/html`, `application/xhtml+xml` | Native Rust (html-to-markdown-rs) | No | Converts to Markdown, metadata extraction |
| XML | `.xml` | `application/xml`, `text/xml` | Native Rust (quick-xml streaming) | No | Element counting, unique element tracking |
| SVG | `.svg` | `image/svg+xml` | Native Rust (XML parser) | No | Treated as XML document |
| reStructuredText | `.rst` | `text/x-rst` | Pandoc | No | Full reST syntax support |
| Org Mode | `.org` | `text/x-org` | Pandoc | No | Emacs Org mode support |
| Rich Text Format | `.rtf` | `application/rtf`, `text/rtf` | Pandoc | No | RTF 1.x support |

### Structured Data

| Format | Extensions | MIME Type | Extraction Method | OCR Support | Special Features |
|--------|-----------|-----------|-------------------|-------------|------------------|
| JSON | `.json` | `application/json`, `text/json` | Native Rust (serde_json) | No | Field counting, nested structure extraction |
| YAML | `.yaml` | `application/x-yaml`, `text/yaml`, `text/x-yaml` | Native Rust (serde_yaml) | No | Multi-document support, field counting |
| TOML | `.toml` | `application/toml`, `text/toml` | Native Rust (toml crate) | No | Configuration file support |
| CSV | `.csv` | `text/csv` | Native Rust (via Pandoc) | No | Tabular data extraction |
| TSV | `.tsv` | `text/tab-separated-values` | Native Rust (via Pandoc) | No | Tab-separated data extraction |

### Email

| Format | Extensions | MIME Type | Extraction Method | OCR Support | Special Features |
|--------|-----------|-----------|-------------------|-------------|------------------|
| EML | `.eml` | `message/rfc822` | Native Rust (mail-parser) | No | Header extraction, attachment listing, body text |
| MSG | `.msg` | `application/vnd.ms-outlook` | Native Rust (mail-parser) | No | Outlook message support, metadata extraction |

### Images

All image formats support OCR when configured with `ocr` parameter in `ExtractionConfig`.

| Format | Extensions | MIME Type | Extraction Method | OCR Support | Special Features |
|--------|-----------|-----------|-------------------|-------------|------------------|
| PNG | `.png` | `image/png` | Native Rust (image-rs) | Yes | EXIF metadata extraction |
| JPEG | `.jpg`, `.jpeg` | `image/jpeg`, `image/jpg` | Native Rust (image-rs) | Yes | EXIF metadata extraction |
| WebP | `.webp` | `image/webp` | Native Rust (image-rs) | Yes | Modern format support |
| BMP | `.bmp` | `image/bmp`, `image/x-bmp`, `image/x-ms-bmp` | Native Rust (image-rs) | Yes | Uncompressed format |
| TIFF | `.tiff`, `.tif` | `image/tiff`, `image/x-tiff` | Native Rust (image-rs) | Yes | Multi-page support |
| GIF | `.gif` | `image/gif` | Native Rust (image-rs) | Yes | Animation frame extraction |
| JPEG 2000 | `.jp2`, `.jpx`, `.jpm`, `.mj2` | `image/jp2`, `image/jpx`, `image/jpm`, `image/mj2` | Native Rust (image-rs) | Yes | Advanced JPEG format |
| PNM Family | `.pnm`, `.pbm`, `.pgm`, `.ppm` | `image/x-portable-anymap`, etc. | Native Rust (image-rs) | Yes | NetPBM formats |

### Archives

| Format | Extensions | MIME Type | Extraction Method | OCR Support | Special Features |
|--------|-----------|-----------|-------------------|-------------|------------------|
| ZIP | `.zip` | `application/zip`, `application/x-zip-compressed` | Native Rust (zip crate) | No | File listing, text content extraction |
| TAR | `.tar`, `.tgz` | `application/x-tar`, `application/tar`, `application/x-gtar`, `application/x-ustar` | Native Rust (tar crate) | No | Unix archive support, compression detection |
| 7-Zip | `.7z` | `application/x-7z-compressed` | Native Rust (sevenz-rust) | No | High compression format support |
| Gzip | `.gz` | `application/gzip` | Native Rust | No | Gzip compression support |

### Academic & Publishing (via Pandoc)

| Format | Extensions | MIME Type | Extraction Method | OCR Support | Special Features |
|--------|-----------|-----------|-------------------|-------------|------------------|
| LaTeX | `.tex`, `.latex` | `application/x-latex`, `text/x-tex` | Pandoc | No | Full LaTeX document support |
| EPUB | `.epub` | `application/epub+zip` | Pandoc | No | E-book format, metadata extraction |
| BibTeX | `.bib` | `application/x-bibtex`, `application/x-biblatex` | Pandoc | No | Bibliography database support |
| Typst | `.typst` | `application/x-typst` | Pandoc | No | Modern typesetting format |
| Jupyter Notebook | `.ipynb` | `application/x-ipynb+json` | Pandoc | No | Code cells, markdown cells, output extraction |
| FictionBook | - | `application/x-fictionbook+xml` | Pandoc | No | XML-based e-book format |
| DocBook | - | `application/docbook+xml` | Pandoc | No | Technical documentation format |
| JATS | - | `application/x-jats+xml` | Pandoc | No | Journal article XML format |
| OPML | - | `application/x-opml+xml` | Pandoc | No | Outline format |
| RIS | - | `application/x-research-info-systems` | Pandoc | No | Citation format |
| EndNote XML | - | `application/x-endnote+xml` | Pandoc | No | Reference manager format |
| CSL JSON | - | `application/csl+json` | Pandoc | No | Citation Style Language JSON |

### Markdown Variants (via Pandoc)

| Format | MIME Type | Extraction Method | Special Features |
|--------|-----------|-------------------|------------------|
| CommonMark | `text/x-commonmark` | Pandoc | Standard Markdown spec |
| GitHub Flavored Markdown | `text/x-gfm` | Pandoc | GFM extensions (tables, strikethrough, etc.) |
| MultiMarkdown | `text/x-multimarkdown` | Pandoc | MMD extensions |
| Markdown Extra | `text/x-markdown-extra` | Pandoc | PHP Markdown Extra extensions |

### Other Formats

| Format | MIME Type | Extraction Method | Special Features |
|--------|-----------|-------------------|------------------|
| Man Pages | `text/x-mdoc` | Pandoc | Unix manual page format |
| Troff | `text/troff` | Pandoc | Unix document format |
| POD | `text/x-pod` | Pandoc | Perl documentation format |
| DokuWiki | `text/x-dokuwiki` | Pandoc | Wiki markup format |

## Architecture Diagram

```mermaid
graph TD
    A[File Input] --> B{MIME Detection}
    B --> C{Extraction Method}

    C -->|Native Format| D[Rust Core Extractors]
    C -->|Pandoc Format| E[Pandoc Subprocess]
    C -->|Legacy Office| F[LibreOffice Conversion]

    D --> G[PDF Extractor]
    D --> H[Excel Extractor]
    D --> I[Image Extractor]
    D --> J[XML/Text/HTML Extractors]
    D --> K[Email Extractor]
    D --> L[Archive Extractor]

    E --> M[DOCX/ODT/EPUB/LaTeX]

    F --> N[Convert DOC→DOCX]
    F --> O[Convert PPT→PPTX]
    N --> E
    O --> D

    G --> P{OCR Needed?}
    I --> P
    P -->|Yes| Q[Tesseract OCR]
    P -->|No| R[Text Output]
    Q --> R

    H --> R
    J --> R
    K --> R
    L --> R
    M --> R

    R --> S[Post-Processing Pipeline]
    S --> T[Final Result]
```

## Feature Flags

Kreuzberg uses Cargo feature flags to enable optional format support:

| Feature Flag | Formats Enabled | Default |
|-------------|----------------|---------|
| `pdf` | PDF documents | No |
| `excel` | Excel spreadsheets (all variants) | No |
| `office` | PowerPoint, Pandoc formats | No |
| `ocr` | OCR for images and PDFs | No |
| `email` | EML, MSG email formats | No |
| `html` | HTML to Markdown conversion | No |
| `xml` | XML document parsing | No |
| `archives` | ZIP, TAR, 7z archive support | No |

**Note:** No features are enabled by default (`default = []`). You must explicitly enable the features you need.

To enable specific features:

```toml
[dependencies]
kreuzberg = { version = "4.0", features = ["pdf", "excel"] }
```

To enable all features with `--all-features`:

```bash
cargo build --all-features
```

Or use the convenience bundles:

```toml
[dependencies]
# All format extraction features (no server)
kreuzberg = { version = "4.0", features = ["full"] }

# Server features (API, MCP) with common formats
kreuzberg = { version = "4.0", features = ["server"] }

# CLI features with common formats
kreuzberg = { version = "4.0", features = ["cli"] }
```

## System Dependencies

Some formats require external system tools:

### Tesseract OCR (Optional)

Required for OCR on images and PDFs:

```bash
# macOS
brew install tesseract

# Ubuntu/Debian
sudo apt-get install tesseract-ocr

# RHEL/CentOS/Fedora
sudo dnf install tesseract

# Windows (Scoop)
scoop install tesseract
```

### Pandoc (Optional)

Required for academic and publishing formats (DOCX, EPUB, LaTeX, etc.):

```bash
# macOS
brew install pandoc

# Ubuntu/Debian
sudo apt-get install pandoc

# RHEL/CentOS/Fedora
sudo dnf install pandoc

# Windows (Scoop)
scoop install pandoc
```

Minimum version: Pandoc 2.x or later

### LibreOffice (Optional)

Required for legacy Microsoft Office formats (`.doc`, `.ppt`):

```bash
# macOS
brew install libreoffice

# Ubuntu/Debian
sudo apt-get install libreoffice

# RHEL/CentOS/Fedora
sudo dnf install libreoffice

# Windows
# Download from https://www.libreoffice.org/download/
```

**Docker Note**: All system dependencies are pre-installed in official Kreuzberg Docker images.

## Format Detection

Kreuzberg automatically detects file formats using:

1. **File Extension Mapping**: 56 formats mapped to MIME types
2. **mime_guess Crate**: Fallback for unknown extensions
3. **Manual Override**: Explicit MIME type can be provided

Example with manual override:

=== "C#"

    ```csharp
    using Kreuzberg;

    // Auto-detect from extension
    var result = KreuzbergClient.ExtractFileSync("document.pdf");

    // Manual MIME type override
    var result2 = KreuzbergClient.ExtractFileAsBytes(rawBytes, "application/pdf", null);
    ```

=== "Go"

    ```go
    import "kreuzberg"

    // Auto-detect from extension
    result, err := kreuzberg.ExtractFileSync("document.pdf", nil)
    if err != nil {
        log.Fatal(err)
    }

    // Manual MIME type override
    config := &kreuzberg.ExtractionConfig{}
    mimeBytes, _ := ioutil.ReadFile("document.dat")
    result2, err := kreuzberg.ExtractBytesSync(mimeBytes, "application/pdf", config)
    ```

=== "Java"

    ```java
    import dev.kreuzberg.Kreuzberg;
    import dev.kreuzberg.ExtractionResult;

    // Auto-detect from extension
    ExtractionResult result = Kreuzberg.extractFile("document.pdf");

    // Manual MIME type override using detectMimeType
    String mimeType = Kreuzberg.detectMimeType(new byte[]{/* PDF header bytes */});
    ExtractionResult result2 = Kreuzberg.extractFileAsBytes(rawBytes, mimeType, null);
    ```

=== "Python"

    ```python
    from kreuzberg import extract_file

    # Auto-detect from extension
    result = extract_file("document.pdf")

    # Manual MIME type override
    result = extract_file("document.dat", mime_type="application/pdf")
    ```

=== "Ruby"

    ```ruby
    require 'kreuzberg'

    # Auto-detect from extension
    result = Kreuzberg.extract_file_sync('document.pdf')

    # Manual MIME type override
    config = Kreuzberg::Config::Extraction.new
    result = Kreuzberg.extract_file_sync('document.dat', mime_type: 'application/pdf', config: config)
    ```

=== "Rust"

    ```rust
    use kreuzberg::{extract_file, ExtractionConfig};

    #[tokio::main]
    async fn main() -> kreuzberg::Result<()> {
        let config = ExtractionConfig::default();

        // Auto-detect from extension
        let result = extract_file("document.pdf", None, &config).await?;

        // Manual MIME type override
        let result = extract_file("document.dat", Some("application/pdf"), &config).await?;

        Ok(())
    }
    ```

=== "TypeScript"

    ```typescript
    import { extractFile } from '@kreuzberg/node';

    // Auto-detect from extension
    const result = await extractFile('document.pdf');

    // Manual MIME type override
    const result2 = await extractFile('document.dat', { mimeType: 'application/pdf' });
    ```

## OCR Support

OCR is available for:

- All image formats (PNG, JPEG, WebP, BMP, TIFF, GIF, etc.)
- PDF documents (with automatic fallback for scanned PDFs)
- Embedded images in PowerPoint presentations

### Configuration

```python
from kreuzberg import extract_file, ExtractionConfig, OcrConfig, TesseractConfig

config = ExtractionConfig(
    ocr=OcrConfig(
        tesseract_config=TesseractConfig(
            lang="eng+deu",  # Multiple languages
            psm=3,           # Page segmentation mode
            oem=1            # OCR Engine mode
        )
    ),
    force_ocr=False  # Only use OCR when native text is insufficient
)

result = extract_file("scanned_document.pdf", config=config)
```

### Automatic OCR Decision

For PDFs, Kreuzberg automatically decides whether OCR is needed by analyzing native text:

- **No OCR**: Document has substantial, meaningful text (>64 non-whitespace chars, >32 chars/page average)
- **OCR Fallback**: Document appears scanned (mostly punctuation, very low alphanumeric ratio)

Override with `force_ocr=True` to always use OCR regardless of native text quality.

## Performance Characteristics

### Native Rust Extractors

- **PDF**: Significantly faster than Python libraries due to native Rust implementation
- **Excel**: Streaming parser, handles multi-GB files
- **XML**: Streaming parser, memory-efficient for large documents
- **Text/Markdown**: Streaming parser with lazy regex compilation
- **Archives**: Efficient extraction without full decompression

### Pandoc Extractors

- Subprocess overhead (~50-200ms per file)
- Good for batch processing with concurrent execution
- Memory-efficient for large documents

### LibreOffice Extractors

- Higher overhead (~500-2000ms per file)
- Only used for legacy formats (`.doc`, `.ppt`)
- Automatic conversion to modern formats

### Batch Processing

All formats support concurrent batch processing:

```python
from kreuzberg import batch_extract_file, ExtractionConfig

paths = ["file1.pdf", "file2.docx", "file3.xlsx"]
config = ExtractionConfig(max_concurrent_extractions=8)

results = batch_extract_file(paths, config=config)
```

## Format Limitations

### Known Limitations

- **Password-Protected PDFs**: Requires `crypto` extra (`pip install kreuzberg[crypto]`)
- **Legacy Excel (.xls)**: Formula evaluation not supported (values only)
- **Encrypted Office Documents**: Password protection not supported
- **Multi-page TIFF**: OCR processes first page only (configurable)
- **Animated GIF**: Extracts first frame only

### Unsupported Formats

- Video formats (MP4, AVI, MOV, etc.)
- Audio formats (MP3, WAV, FLAC, etc.)
- CAD formats (DWG, DXF, etc.)
- Database files (MDB, ACCDB, etc.)
- Compressed Office formats without proper headers

## Adding New Formats

Kreuzberg's plugin system allows adding custom format extractors:

=== "C#"

    ```csharp
    using Kreuzberg;
    using Kreuzberg.Plugins;

    public class CustomExtractor : IDocumentExtractor
    {
        public string Name => "custom-format-extractor";

        public string[] SupportedMimeTypes => new[] { "application/x-custom" };

        public ExtractionResult ExtractBytes(byte[] content, string mimeType, ExtractionConfig config)
        {
            // Your extraction logic here
            var text = ParseCustomFormat(content);
            return new ExtractionResult
            {
                Content = text,
                MimeType = mimeType,
                Metadata = new Dictionary<string, object>()
            };
        }
    }

    // Register plugin
    KreuzbergClient.RegisterDocumentExtractor(new CustomExtractor());
    ```

=== "Go"

    ```go
    package main

    import (
        "kreuzberg"
        "log"
    )

    // DocumentExtractor implementation
    type CustomExtractor struct{}

    func (e *CustomExtractor) Name() string {
        return "custom-format-extractor"
    }

    func (e *CustomExtractor) SupportedMimeTypes() []string {
        return []string{"application/x-custom"}
    }

    func (e *CustomExtractor) ExtractBytes(content []byte, mimeType string, config *kreuzberg.ExtractionConfig) (*kreuzberg.ExtractionResult, error) {
        // Your extraction logic here
        text := parseCustomFormat(content)
        return &kreuzberg.ExtractionResult{
            Content:  text,
            MimeType: mimeType,
            Success:  true,
        }, nil
    }

    // Register plugin
    func init() {
        if err := kreuzberg.RegisterDocumentExtractor("custom-format-extractor", &CustomExtractor{}); err != nil {
            log.Fatal(err)
        }
    }
    ```

=== "Java"

    ```java
    import dev.kreuzberg.Kreuzberg;
    import dev.kreuzberg.DocumentExtractorProtocol;
    import dev.kreuzberg.ExtractionResult;
    import dev.kreuzberg.config.ExtractionConfig;

    public class CustomExtractor implements DocumentExtractorProtocol {
        @Override
        public String name() {
            return "custom-format-extractor";
        }

        @Override
        public String[] supportedMimeTypes() {
            return new String[]{"application/x-custom"};
        }

        @Override
        public ExtractionResult extractBytes(
            byte[] content,
            String mimeType,
            ExtractionConfig config) throws Exception {
            // Your extraction logic here
            String text = parseCustomFormat(content);
            return new ExtractionResult(text, mimeType, true, null);
        }
    }

    // Register plugin
    Kreuzberg.registerDocumentExtractor(new CustomExtractor());
    ```

=== "Python"

    ```python
    from kreuzberg import DocumentExtractor, ExtractionResult, Metadata

    class CustomExtractor(DocumentExtractor):
        def name(self) -> str:
            return "custom-format-extractor"

        def supported_mime_types(self) -> list[str]:
            return ["application/x-custom"]

        def extract_bytes(self, content: bytes, mime_type: str, config) -> ExtractionResult:
            # Your extraction logic here
            text = parse_custom_format(content)
            return ExtractionResult(
                content=text,
                mime_type=mime_type,
                metadata=Metadata()
            )

    # Register plugin
    from kreuzberg import get_document_extractor_registry
    registry = get_document_extractor_registry()
    registry.register(CustomExtractor())
    ```

=== "Ruby"

    ```ruby
    require 'kreuzberg'

    class CustomExtractor
      def name
        'custom-format-extractor'
      end

      def supported_mime_types
        ['application/x-custom']
      end

      def extract_bytes(content, mime_type, config)
        # Your extraction logic here
        text = parse_custom_format(content)
        Kreuzberg::Result.new(
          content: text,
          mime_type: mime_type,
          metadata: {}
        )
      end
    end

    # Register plugin
    Kreuzberg.register_document_extractor(CustomExtractor.new)
    ```

=== "Rust"

    ```rust
    use kreuzberg::plugins::{DocumentExtractor, Plugin};
    use kreuzberg::types::ExtractionResult;
    use async_trait::async_trait;

    pub struct CustomExtractor;

    impl Plugin for CustomExtractor {
        fn name(&self) -> &str {
            "custom-format-extractor"
        }

        fn version(&self) -> String {
            "1.0.0".to_string()
        }
    }

    #[async_trait]
    impl DocumentExtractor for CustomExtractor {
        async fn extract_bytes(
            &self,
            content: &[u8],
            mime_type: &str,
            config: &ExtractionConfig,
        ) -> kreuzberg::Result<ExtractionResult> {
            // Your extraction logic here
            let text = parse_custom_format(content)?;
            Ok(ExtractionResult {
                content: text,
                mime_type: mime_type.to_string(),
                ..Default::default()
            })
        }

        fn supported_mime_types(&self) -> &[&str] {
            &["application/x-custom"]
        }
    }

    // Register plugin
    use kreuzberg::plugins::registry::get_document_extractor_registry;
    use std::sync::Arc;

    let registry = get_document_extractor_registry();
    registry.write().unwrap().register(Arc::new(CustomExtractor))?;
    ```

=== "TypeScript"

    ```typescript
    import { registerDocumentExtractor, type DocumentExtractorProtocol } from '@kreuzberg/node';

    class CustomExtractor implements DocumentExtractorProtocol {
        name(): string {
            return "custom-format-extractor";
        }

        supportedMimeTypes(): string[] {
            return ["application/x-custom"];
        }

        async extractBytes(content: Uint8Array, mimeType: string, config?: ExtractionConfig): Promise<ExtractionResult> {
            // Your extraction logic here
            const text = parseCustomFormat(content);
            return {
                content: text,
                mimeType: mimeType,
                success: true,
                metadata: {}
            };
        }
    }

    // Register plugin
    registerDocumentExtractor(new CustomExtractor());
    ```

## See Also

- [Configuration Reference](configuration.md) - Detailed configuration options
- [Extraction Guide](../guides/extraction.md) - Extraction examples
- [OCR Guide](../guides/ocr.md) - OCR configuration and usage
- [Plugin System](../concepts/plugin-system.md) - Custom extractor development
