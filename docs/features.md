# Features

Kreuzberg is a comprehensive document intelligence library supporting 56 file formats with advanced extraction, OCR, and processing capabilities. This page documents all features and their availability across language bindings.

## Core Extraction Features

### File Format Support

Kreuzberg extracts text, tables, and metadata from 56 file formats:

**Documents**
- PDF (`.pdf`) - Native text extraction with optional OCR fallback
- Microsoft Word (`.docx`, `.doc`) - Modern and legacy formats
- LibreOffice Writer (`.odt`) - OpenDocument text
- Plain text (`.txt`, `.md`, `.markdown`) - With metadata extraction for Markdown

**Spreadsheets**
- Excel (`.xlsx`, `.xls`, `.xlsm`, `.xlsb`) - Modern and legacy formats
- LibreOffice Calc (`.ods`) - OpenDocument spreadsheet
- CSV (`.csv`) - Comma-separated values
- TSV (`.tsv`) - Tab-separated values

**Presentations**
- PowerPoint (`.pptx`, `.ppt`) - Modern and legacy formats

**Images**
- Common formats: JPEG, PNG, GIF, BMP, TIFF, WebP
- Advanced formats: JPEG 2000 (`.jp2`, `.jpx`, `.jpm`, `.mj2`)
- Portable formats: PNM, PBM, PGM, PPM

**Email**
- EML (`.eml`) - RFC 822 email format
- MSG (`.msg`) - Microsoft Outlook format

**Web & Markup**
- HTML (`.html`, `.htm`) - Converted to Markdown
- XML (`.xml`) - Streaming parser for large files
- SVG (`.svg`) - Scalable vector graphics

**Structured Data**
- JSON (`.json`) - JavaScript Object Notation
- YAML (`.yaml`) - YAML Ain't Markup Language
- TOML (`.toml`) - Tom's Obvious Minimal Language

**Archives**
- ZIP (`.zip`) - ZIP archives
- TAR (`.tar`, `.tgz`) - Tape archives
- GZIP (`.gz`) - GNU zip
- 7-Zip (`.7z`) - 7-Zip archives

### Extraction Capabilities

**Text Extraction**
- Native text extraction from all supported formats
- Preserves formatting and structure where applicable
- Handles multi-byte character encodings (UTF-8, UTF-16, etc.)
- Mojibake detection and correction

**Table Extraction**
- Structured table data from PDFs, spreadsheets, and Word documents
- Cell-level extraction with row/column indexing
- Markdown and JSON output formats
- Merged cell support

**Metadata Extraction**
- Document properties (title, author, creation date, etc.)
- Page count, word count, character count
- MIME type detection
- Format-specific metadata (Excel sheets, PDF annotations, etc.)

**Image Extraction**
- Extract embedded images from PDFs and Office documents
- Image preprocessing for OCR optimization
- Format conversion and resolution normalization

## OCR (Optical Character Recognition)

### Tesseract OCR

Native Tesseract integration available in all language bindings.

**Features:**
- 100+ language support via Tesseract language packs
- Page segmentation modes (PSM) for different layouts
- OCR Engine Modes (OEM) for accuracy tuning
- Confidence scoring per word/line
- hOCR output format support
- Automatic image preprocessing

**Configuration:**
- Language selection (single or multi-language)
- PSM and OEM mode selection
- Custom Tesseract configuration strings
- Whitelist/blacklist character sets

### Python-Specific OCR Backends

Python bindings provide two additional OCR backends via optional dependencies:

**EasyOCR** (`pip install kreuzberg[easyocr]`)
- Deep learning-based OCR engine
- 80+ language support
- GPU acceleration support (CUDA)
- Better accuracy for certain scripts (CJK, Arabic, etc.)
- Requires Python <3.14

**PaddleOCR** (`pip install kreuzberg[paddleocr]`)
- Production-ready OCR from PaddlePaddle
- Ultra-lightweight models
- 80+ language support
- Mobile deployment capability
- Requires Python <3.14

### OCR Features

- **Automatic fallback**: Use OCR when native text extraction fails
- **Force OCR mode**: Override native extraction with OCR
- **Caching**: OCR results cached to disk for performance
- **Image preprocessing**: Automatic contrast, deskew, and noise reduction
- **Multi-language detection**: Process documents with mixed languages

## Advanced Processing Features

### Language Detection

Automatic language detection for extracted text using fast-langdetect.

**Capabilities:**
- 60+ language detection
- Confidence scoring
- Multi-language detection (detect all languages in document)
- Configurable confidence thresholds
- ISO 639-1 and ISO 639-3 code support

**Configuration:**
```python
LanguageDetectionConfig(
    detect_multiple=True,
    confidence_threshold=0.7
)
```

### Content Chunking

Split extracted text into semantic chunks for LLM processing.

**Chunking Strategies:**
- **Recursive**: Split by paragraphs, sentences, then words
- **Semantic**: Preserve semantic boundaries
- **Token-aware**: Respect token limits for LLMs

**Features:**
- Configurable chunk size and overlap
- Metadata preservation per chunk
- Character position tracking
- Optional embedding generation

**Configuration:**
```python
ChunkingConfig(
    max_chars=1000,
    max_overlap=200
)
```

### Embeddings

Generate vector embeddings for chunks using FastEmbed.

**Embedding Models:**
- **Preset models**: `"fast"`, `"balanced"`, `"quality"`
- **FastEmbed models**: Any model from FastEmbed catalog
- **Custom models**: Bring your own embedding model

**Features:**
- Local embedding generation (no API calls)
- Automatic model download and caching
- Multiple embedding dimensions (384, 512, 768, 1024)
- Batch processing for performance
- Optional L2 normalization

**Configuration:**
```python
EmbeddingConfig(
    model=EmbeddingModelType.preset("balanced"),
    normalize=True
)
```

### Token Reduction

Reduce token count while preserving semantic meaning using extractive summarization.

**Reduction Modes:**
- **Light** (`"light"`): ~15% reduction, minimal information loss
- **Moderate** (`"moderate"`): ~30% reduction, balanced approach
- **Aggressive** (`"aggressive"`): ~50% reduction, maximum compression

**Algorithm:**
- TF-IDF based sentence scoring
- Stopword filtering with language-specific lists
- Position-aware scoring (preserve important sections)
- Configurable reduction targets

**Configuration:**
```python
TokenReductionConfig(
    mode="moderate"
)
```

### Quality Processing

Enhance extraction quality with text normalization and cleanup.

**Processing Steps:**
- Unicode normalization (NFC/NFD/NFKC/NFKD)
- Whitespace normalization
- Line break standardization
- Encoding detection and correction
- Mojibake fixing
- Character set validation

**Configuration:**
```python
ExtractionConfig(
    enable_quality_processing=True
)
```

### Keyword Extraction

Extract keywords and key phrases from documents.

**Algorithms:**
- **YAKE** (Yet Another Keyword Extractor): Unsupervised, language-independent
- **RAKE** (Rapid Automatic Keyword Extraction): Fast statistical method

**Features:**
- Configurable number of keywords
- N-gram support (1-3 word phrases)
- Language-specific stopword filtering
- Relevance scoring

**Configuration:**
```python
KeywordExtractionConfig(
    algorithm="yake",
    max_keywords=10,
    ngram_size=3
)
```

## Batch Processing

### Parallel Extraction

Process multiple documents concurrently using async/await or thread pools.

**Async API:**
```python
results = await batch_extract_file(
    ["doc1.pdf", "doc2.pdf", "doc3.pdf"],
    config=config
)
```

**Features:**
- Automatic concurrency based on CPU count
- Configurable worker limits
- Error handling per document
- Progress tracking
- Memory-efficient streaming for large batches

### Caching

Intelligent caching system for expensive operations.

**Cached Operations:**
- OCR results (per image hash)
- Language detection results
- Embedding vectors
- Extracted metadata

**Cache Features:**
- Disk-based storage
- Automatic cache invalidation
- Configurable cache directory
- Cache statistics and management
- LRU eviction policy

**Configuration:**
```python
ExtractionConfig(
    use_cache=True,
    cache_dir="/custom/cache/path"
)
```

## Configuration & Discovery

### Configuration Methods

Kreuzberg supports four configuration methods:

1. **Programmatic**: Create configuration objects in code
2. **TOML files**: `kreuzberg.toml`
3. **YAML files**: `kreuzberg.yaml`
4. **JSON files**: `kreuzberg.json`

### Automatic Discovery

Configuration files automatically discovered in order:

1. Current directory: `./kreuzberg.{toml,yaml,json}`
2. User config: `~/.config/kreuzberg/config.{toml,yaml,json}`
3. System config: `/etc/kreuzberg/config.{toml,yaml,json}`

**Discovery API:**
```python
config = ExtractionConfig.discover()
```

### Environment Variables

Override configuration via environment variables:

- `KREUZBERG_CONFIG_PATH`: Path to config file
- `KREUZBERG_CACHE_DIR`: Cache directory
- `KREUZBERG_OCR_BACKEND`: OCR backend selection
- `KREUZBERG_OCR_LANGUAGE`: OCR language

## Plugin System

### Plugin Types

Extensible architecture supporting four plugin types:

**Document Extractors**
- Add support for custom file formats
- Override default extractors
- Priority-based selection

**OCR Backends**
- Integrate cloud OCR services
- Custom OCR engines
- Preprocessing pipelines

**Post Processors**
- Transform extraction results
- Add custom metadata
- Filter or enhance content

**Validators**
- Validate extraction results
- Enforce quality standards
- Custom error handling

### Plugin Registration

**Rust:**
```rust
let registry = get_document_extractor_registry();
registry.register("custom", Arc::new(MyExtractor), 50)?;
```

**Python:**
```python
from kreuzberg.plugins import register_extractor

register_extractor(MyExtractor(), priority=50)
```

### Plugin Discovery

Automatic plugin discovery from:
- Python entry points
- Configuration files
- Environment variables

## Server Modes

### HTTP REST API Server

Production-ready RESTful API server.

**Endpoints:**
- `POST /extract` - Extract from uploaded files
- `GET /health` - Health check
- `GET /info` - Server information
- `GET /cache/stats` - Cache statistics
- `POST /cache/clear` - Clear cache

**Features:**
- File upload support
- JSON/multipart request handling
- CORS configuration
- Request logging and metrics
- Graceful shutdown

**Start Server:**
```bash
kreuzberg serve --host 0.0.0.0 --port 8000
```

### Model Context Protocol (MCP) Server

Stdio-based MCP server for AI agent integration.

**Tools:**
- `extract_file` - Extract from file path
- `extract_bytes` - Extract from base64 bytes
- `batch_extract` - Extract from multiple files

**Features:**
- Stdio transport (Claude Desktop, Continue.dev, etc.)
- JSON-RPC 2.0 protocol
- Streaming results
- Error handling

**Start Server:**
```bash
kreuzberg mcp
```

**Claude Desktop Configuration:**
```json
{
  "mcpServers": {
    "kreuzberg": {
      "command": "kreuzberg",
      "args": ["mcp"]
    }
  }
}
```

## Language Binding Comparison

### Feature Availability

| Feature | C# | Go | Python | Ruby | Rust | TypeScript |
|---------|----|----|--------|------|------|------------|
| **Core Extraction** | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| All file formats | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| Table extraction | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| Metadata extraction | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| **OCR** | | | | | | |
| Tesseract | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| EasyOCR | ✗ | ✗ | ✓ (optional) | ✗ | ✗ | ✗ |
| PaddleOCR | ✗ | ✗ | ✓ (optional) | ✗ | ✗ | ✗ |
| **Processing** | | | | | | |
| Language detection | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| Content chunking | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| Embeddings | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| Token reduction | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| Quality processing | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| Keyword extraction | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| **Configuration** | | | | | | |
| Programmatic config | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| File-based config | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| Config discovery | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| **Plugin System** | | | | | | |
| Document extractors | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| OCR backends | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| Post processors | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| Validators | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| **Servers** | | | | | | |
| HTTP REST API | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| MCP Server | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| **APIs** | | | | | | |
| Sync API | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| Async API | ✗ | ✗ | ✓ | ✗ | ✓ | ✓ |
| Batch processing | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| Streaming | ✗ | ✗ | ✗ | ✗ | ✓ | ✗ |

### Package Distribution

| Language | Package Manager | Modular Features | Full Package |
|----------|----------------|------------------|--------------|
| C# | NuGet | ✗ | ✓ (default) |
| Go | go.pkg.dev | ✗ | ✓ (default) |
| Python | PyPI (`pip`) | ✗ | ✓ (default) |
| Ruby | RubyGems (`gem`) | ✗ | ✓ (default) |
| Rust | crates.io | ✓ | ✗ (opt-in) |
| TypeScript | npm | ✗ | ✓ (default) |

### Rust Feature Flags

Rust provides fine-grained control over included components via Cargo features:

**Format Extractors:**
- `pdf` - PDF extraction (pdfium)
- `excel` - Excel/spreadsheet support
- `office` - Office document support (Word, PowerPoint)
- `email` - Email extraction (EML, MSG)
- `html` - HTML to Markdown conversion
- `xml` - XML streaming parser
- `archives` - Archive extraction (ZIP, TAR, 7z)

**Processing Features:**
- `ocr` - Tesseract OCR integration
- `language-detection` - Language detection
- `chunking` - Content chunking
- `embeddings` - Embedding generation (requires `chunking`)
- `quality` - Quality processing and text normalization
- `keywords` - Keyword extraction (YAKE + RAKE)
- `stopwords` - Stopword filtering

**Server Features:**
- `api` - HTTP REST API server
- `mcp` - Model Context Protocol server

**Convenience Bundles:**
- `full` - All format extractors + all processing features
- `server` - Server features + common extractors
- `cli` - CLI features + common extractors

**Example Cargo.toml:**
```toml
[dependencies]
kreuzberg = { version = "4.0", features = ["pdf", "ocr", "chunking"] }
```

**Default:** No features enabled (minimal build)

### Python Optional Dependencies

Python bindings include all core features by default. Optional OCR backends require separate installation:

```bash
# Core package (Tesseract OCR only)
pip install kreuzberg

# With EasyOCR
pip install kreuzberg[easyocr]

# With PaddleOCR
pip install kreuzberg[paddleocr]

# All optional features
pip install kreuzberg[all]
```

**Note:** EasyOCR and PaddleOCR require Python <3.14 due to PyTorch dependencies.

### TypeScript/Ruby Packages

TypeScript and Ruby bindings include all features in a single package. No optional dependencies or modular installation.

```bash
# TypeScript - full package
npm install kreuzberg

# Ruby - full package
gem install kreuzberg
```

## Performance Characteristics

### Rust Core

Kreuzberg's Rust core provides efficient performance for document processing:

**Key Optimizations:**
- Streaming parsers for large documents
- Concurrent extraction with configurable worker pools
- Intelligent caching of expensive operations

### Memory Efficiency

**Streaming Support:**
- XML: Constant memory regardless of file size
- Plain text: Line-by-line streaming for large files
- Archives: Extract on-demand without loading entire archive

### Caching

Disk-based caching improves performance for repeated operations:

- Cache results for OCR, language detection, and embeddings
- Automatic cache invalidation based on file content hash
- Configurable cache directory and eviction policy

## CLI Tools

### Extract Command

Primary CLI for document extraction.

```bash
kreuzberg extract document.pdf --ocr --format json
```

**Features:**
- Batch processing with glob patterns
- Parallel processing (`--parallel`)
- Output format selection (text, JSON)
- OCR configuration
- Progress reporting

### Serve Command

Start HTTP REST API server.

```bash
kreuzberg serve --host 0.0.0.0 --port 8000 --config production.toml
```

### MCP Command

Start Model Context Protocol server.

```bash
kreuzberg mcp --config kreuzberg.toml
```

### Cache Management

```bash
# View cache statistics
kreuzberg cache stats

# Clear cache
kreuzberg cache clear
```

See [CLI Usage](cli/usage.md) for complete documentation.

## System Requirements

### Runtime Dependencies

**Required (all platforms):**
- Tesseract OCR (4.0+) for OCR functionality
- LibreOffice (optional) for legacy Office formats (.doc, .ppt)
- Pandoc (optional) for advanced document conversion

**Installation:**
```bash
# macOS
brew install tesseract libreoffice pandoc

# Ubuntu/Debian
apt-get install tesseract-ocr libreoffice pandoc

# RHEL/CentOS/Fedora
dnf install tesseract libreoffice pandoc

# Windows (Chocolatey)
choco install tesseract libreoffice pandoc
```

### Python Requirements

- Python 3.10+
- Optional: CUDA toolkit for GPU-accelerated OCR (EasyOCR)

### TypeScript/Node.js Requirements

- Node.js 18+
- Native module support (node-gyp)

### Rust Requirements

- Rust 1.80+ (edition 2024)
- Cargo for building from source

### Ruby Requirements

- Ruby 3.3+
- Native extension support

## Docker Images

Pre-built Docker images available on Docker Hub:

**Variants:**
- `goldziher/kreuzberg:latest` - Core + Tesseract
- `goldziher/kreuzberg:latest-all` - All features

**Usage:**
```bash
docker run -v $(pwd):/data goldziher/kreuzberg:latest \
  extract /data/document.pdf --ocr
```

See [Installation Guide](getting-started/installation.md) for detailed instructions.

## Next Steps

- [Installation](getting-started/installation.md) - Install Kreuzberg
- [Quick Start](getting-started/quickstart.md) - Get started in 5 minutes
- [Configuration](guides/configuration.md) - Configure extraction behavior
- [API Reference](reference/api-python.md) - Complete API documentation
