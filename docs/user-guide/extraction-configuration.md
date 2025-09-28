# Extraction Configuration

Kreuzberg provides extensive configuration options for the extraction process through the `ExtractionConfig` class. You can configure Kreuzberg either programmatically through code or via configuration files that are automatically discovered. This guide covers both approaches and common configuration scenarios.

## Configuration Files (Recommended)

Kreuzberg automatically discovers and loads configuration from files in your project directory. This is the recommended approach for consistent configuration across your project.

### Supported Configuration Files

Kreuzberg searches for configuration files in the following order:

1. `kreuzberg.toml` - Dedicated configuration file (recommended)
1. `pyproject.toml` with `[tool.kreuzberg]` section

The search starts from the current working directory and walks up the directory tree until a configuration file is found.

### kreuzberg.toml Example

Create a `kreuzberg.toml` file in your project root:

```toml
# Basic extraction settings
force_ocr = false
chunk_content = true
extract_tables = true
extract_entities = false
extract_keywords = true
keyword_count = 15
max_chars = 2000
max_overlap = 100
ocr_backend = "tesseract"
auto_detect_language = true
auto_detect_document_type = true
document_classification_mode = "text"  # or "vision"
type_confidence_threshold = 0.5

# Image extraction configuration
extract_images = true              # Extract embedded images from documents
deduplicate_images = true          # Remove duplicate images

# Image OCR configuration
[image_ocr_config]
enabled = true                     # Run OCR on extracted images
backend = "tesseract"              # OCR engine for images
min_dimensions = [100, 100]        # Minimum image dimensions for OCR
max_dimensions = [5000, 5000]      # Maximum image dimensions for OCR
batch_size = 10                    # Number of images to process in parallel

# Tesseract OCR configuration
[tesseract]
language = "eng+deu"  # English and German
psm = 6               # Uniform block of text

# EasyOCR configuration (if using easyocr backend)
[easyocr]
language_list = ["en", "de"]
gpu = false

# PaddleOCR configuration (if using paddleocr backend)
[paddleocr]
language = "en"
use_gpu = false

# Table extraction configuration (GMFT)
[gmft]
verbosity = 1
detection_threshold = 0.7
structure_threshold = 0.5
# Uses optimized model defaults
detection_device = "auto"
structure_device = "auto"
enable_model_caching = true

# DPI and Image Processing configuration
target_dpi = 150                  # Target DPI for document processing
max_image_dimension = 25000       # Maximum pixel dimension before auto-scaling
auto_adjust_dpi = true           # Automatically adjust DPI for large documents
min_dpi = 72                     # Minimum DPI threshold
max_dpi = 600                    # Maximum DPI threshold

# Language detection configuration
[language_detection]
multilingual = true
top_k = 3
low_memory = false

# Entity extraction configuration (spaCy)
[spacy_entity_extraction]
language_models = { en = "en_core_web_sm", de = "de_core_news_sm" }
fallback_to_multilingual = true

# HTML to Markdown conversion configuration
[html_to_markdown]
heading_style = "atx"
strong_em_symbol = "_"
escape_underscores = false
wrap = true
wrap_width = 100
list_indent_width = 2                # Use 2 spaces for Discord/Slack compatibility
list_indent_type = "spaces"          # Use spaces instead of tabs
whitespace_mode = "normalized"       # Handle whitespace intelligently
br_in_tables = false                 # Use spaces instead of <br> in tables
highlight_style = "double-equal"     # Style for highlighted text
newline_style = "spaces"             # Style for line breaks
preprocess_html = true               # Clean messy HTML before conversion
preprocessing_preset = "standard"    # Level of HTML cleaning
```

### pyproject.toml Example

Alternatively, add configuration to your existing `pyproject.toml`:

```toml
[tool.kreuzberg]
force_ocr = false
chunk_content = true
extract_tables = true
auto_detect_language = true
auto_detect_document_type = true
document_classification_mode = "text"
type_confidence_threshold = 0.5

# Image extraction configuration
extract_images = true              # Extract embedded images from documents
deduplicate_images = true          # Remove duplicate images

# Image OCR configuration
[image_ocr_config]
enabled = true                     # Run OCR on extracted images
backend = "tesseract"              # OCR engine for images
min_dimensions = [100, 100]        # Minimum image dimensions for OCR
max_dimensions = [5000, 5000]      # Maximum image dimensions for OCR
batch_size = 10                    # Number of images to process in parallel

# DPI and Image Processing
target_dpi = 150
max_image_dimension = 25000
auto_adjust_dpi = true
min_dpi = 72
max_dpi = 600

[tool.kreuzberg.image_ocr_config]
enabled = true                     # Run OCR on extracted images
backend = "tesseract"              # OCR engine for images
min_dimensions = [100, 100]        # Minimum image dimensions for OCR
max_dimensions = [5000, 5000]      # Maximum image dimensions for OCR
batch_size = 10                    # Number of images to process in parallel

[tool.kreuzberg.tesseract]
language = "eng"
psm = 6

[tool.kreuzberg.gmft]
detection_threshold = 0.7
structure_threshold = 0.5
detection_device = "auto"
structure_device = "auto"
```

### Using Configuration Files

Once you have a configuration file, all Kreuzberg functions will automatically use it:

```python
from kreuzberg import extract_file

# Automatically uses configuration from kreuzberg.toml or pyproject.toml
result = await extract_file("document.pdf")

# Configuration is also used by CLI commands
# $ python -m kreuzberg.cli extract document.pdf

# And by the API server
# $ uvicorn kreuzberg._api.main:app
```

### Viewing Current Configuration

You can check what configuration is being used:

```python
from kreuzberg._config import try_discover_config

config = try_discover_config()
if config:
    print(f"Using configuration with OCR backend: {config.ocr_backend}")
    print(f"Table extraction enabled: {config.extract_tables}")
else:
    print("No configuration file found, using defaults")
```

Or using the CLI:

```bash
python -m kreuzberg.cli config
```

### Configuration Priority

When configuration files are present, you can still override specific settings programmatically:

```python
from kreuzberg import extract_file, ExtractionConfig

# Override just the OCR setting while keeping other file-based config
result = await extract_file("document.pdf", config=ExtractionConfig(force_ocr=True))
```

The priority order is:

1. Programmatic configuration (highest priority)
1. Configuration file settings
1. Default values (lowest priority)

## API Runtime Configuration

When using the [Kreuzberg API Server](api-server.md), you can configure extraction behavior at runtime without requiring static configuration files. This allows different requests to use different extraction settings.

### Query Parameters

Configure extraction options directly via URL query parameters when making requests to the `/extract` endpoint:

Enable chunking with custom settings:

```bash
curl -X POST "http://localhost:8000/extract" \
  -F "data=@document.pdf" \
  -F "chunk_content=true" \
  -F "max_chars=500" \
  -F "max_overlap=50"
```

Extract entities and keywords:

```bash
curl -X POST "http://localhost:8000/extract" \
  -F "data=@document.pdf" \
  -F "extract_entities=true" \
  -F "extract_keywords=true" \
  -F "keyword_count=5"
```

Force OCR with specific backend:

```bash
curl -X POST "http://localhost:8000/extract" \
  -F "data=@image.jpg" \
  -F "force_ocr=true" \
  -F "ocr_backend=tesseract"
```

**Supported Query Parameters:**

- `chunk_content` (boolean): Enable content chunking
- `max_chars` (integer): Maximum characters per chunk
- `max_overlap` (integer): Overlap between chunks in characters
- `extract_tables` (boolean): Enable table extraction
- `extract_entities` (boolean): Enable named entity extraction
- `extract_keywords` (boolean): Enable keyword extraction
- `keyword_count` (integer): Number of keywords to extract
- `force_ocr` (boolean): Force OCR processing
- `ocr_backend` (string): OCR engine (`tesseract`, `easyocr`, `paddleocr`)
- `auto_detect_language` (boolean): Enable automatic language detection
- `pdf_password` (string): Password for encrypted PDFs
- `extract_images` (boolean): Extract embedded images from supported formats (PDF, PPTX, HTML, Office)
- `extract_images` (boolean): Extract embedded images from documents
- `deduplicate_images` (boolean): Remove duplicate images by content hash
- Note: For image OCR configuration via API, use the `ImageOCRConfig` in request body instead of query parameters

### Header Configuration

For complex nested configurations (like OCR-specific settings), use the `X-Extraction-Config` header with JSON format:

Advanced OCR configuration:

```bash
curl -X POST "http://localhost:8000/extract" \
  -H "Content-Type: multipart/form-data" \
  -H "X-Extraction-Config: {
    \"force_ocr\": true,
    \"ocr_backend\": \"tesseract\",
    \"ocr_config\": {
      \"language\": \"eng+deu\",
      \"psm\": 6,
      \"output_format\": \"text\"
    }
  }" \
  -F "data=@multilingual_document.pdf"
```

Table extraction with GMFT configuration:

```bash
curl -X POST "http://localhost:8000/extract" \
  -H "Content-Type: multipart/form-data" \
  -H "X-Extraction-Config: {
    \"extract_tables\": true,
    \"gmft_config\": {
      \"detection_threshold\": 0.8,
      \"structure_threshold\": 0.6,
      \"crop_padding\": 25,
      \"min_table_area\": 1000
    }
  }" \
  -F "data=@document_with_tables.pdf"
```

### API Configuration Precedence

When using the API server, configuration is merged with the following precedence:

1. **Header config** (highest priority) - `X-Extraction-Config` header
1. **Query params** - URL query parameters
1. **Static config** - `kreuzberg.toml` or `pyproject.toml` files
1. **Defaults** (lowest priority) - Built-in default values

This means you can have a base configuration in files, override specific settings via query parameters, and use headers for complex nested configuration—all in the same request.

### Mapping API Parameters to Configuration

The runtime API parameters correspond directly to the programmatic configuration options:

| Query Parameter    | Config Class Field                  | Header JSON Key                 |
| ------------------ | ----------------------------------- | ------------------------------- |
| `chunk_content`    | `ExtractionConfig.chunk_content`    | `"chunk_content"`               |
| `max_chars`        | `ExtractionConfig.max_chars`        | `"max_chars"`                   |
| `extract_entities` | `ExtractionConfig.extract_entities` | `"extract_entities"`            |
| `force_ocr`        | `ExtractionConfig.force_ocr`        | `"force_ocr"`                   |
| `ocr_backend`      | `ExtractionConfig.ocr_backend`      | `"ocr_backend"`                 |
| N/A                | `ExtractionConfig.ocr_config`       | `"ocr_config"` (nested object)  |
| N/A                | `ExtractionConfig.gmft_config`      | `"gmft_config"` (nested object) |

For complete API documentation and examples, see the [API Server guide](api-server.md).

## Programmatic Configuration

You can also configure Kreuzberg entirely through code using the `ExtractionConfig` class. This approach gives you full control and is useful for dynamic configuration.

### Basic Configuration

All extraction functions accept an optional `config` parameter of type `ExtractionConfig`. This object allows you to:

- Control OCR behavior with `force_ocr` and `ocr_backend`
- Provide engine-specific OCR configuration via `ocr_config`
- Enable table extraction with `extract_tables` and configure it via `gmft_config`
- Enable automatic language detection with `auto_detect_language`
- Add validation and post-processing hooks
- Configure custom extractors

## Examples

### Basic Usage

```python
from kreuzberg import extract_file, ExtractionConfig

# Simple extraction with default configuration
result = await extract_file("document.pdf")

# Extraction with custom configuration
result = await extract_file("document.pdf", config=ExtractionConfig(force_ocr=True))
```

### OCR Configuration

```python
from kreuzberg import extract_file, ExtractionConfig, TesseractConfig, PSMMode

# Configure Tesseract OCR with specific language and page segmentation mode
result = await extract_file(
    "document.pdf",
    config=ExtractionConfig(force_ocr=True, ocr_config=TesseractConfig(language="eng+deu", psm=PSMMode.SINGLE_BLOCK)),
)
```

The `language` parameter specifies which language model Tesseract should use. You can specify multiple languages by joining them with a plus sign (e.g., "eng+deu" for English and German).

The `psm` (Page Segmentation Mode) parameter controls how Tesseract analyzes page layout. Different modes are suitable for different types of documents:

- `PSMMode.AUTO`: Automatic page segmentation (default)
- `PSMMode.SINGLE_BLOCK`: Treat the image as a single text block
- `PSMMode.SINGLE_LINE`: Treat the image as a single text line
- `PSMMode.SINGLE_WORD`: Treat the image as a single word
- `PSMMode.SINGLE_CHAR`: Treat the image as a single character

### Alternative OCR Engines

```python
from kreuzberg import extract_file, ExtractionConfig, EasyOCRConfig, PaddleOCRConfig

# Use EasyOCR backend
result = await extract_file(
    "document.jpg", config=ExtractionConfig(ocr_backend="easyocr", ocr_config=EasyOCRConfig(language_list=["en", "de"]))
)

# Use PaddleOCR backend
result = await extract_file(
    "chinese_document.jpg", config=ExtractionConfig(ocr_backend="paddleocr", ocr_config=PaddleOCRConfig(language="ch"))
)
```

### Table Extraction

Kreuzberg offers multiple approaches for extracting tables from documents. For detailed information, see the [Table Extraction Guide](table-extraction.md).

#### Quick Configuration

```python
from kreuzberg import extract_file, ExtractionConfig, GMFTConfig

# Vision-based table extraction (recommended for complex tables)
config_ai = ExtractionConfig(extract_tables=True, gmft_config=GMFTConfig(detection_threshold=0.7))

# OCR-based table extraction (lightweight, for simple tables)
config_ocr = ExtractionConfig(extract_tables_from_ocr=True)

# Both methods combined (maximum coverage)
config_both = ExtractionConfig(
    extract_tables=True,
    extract_tables_from_ocr=True,
)

result = await extract_file("document_with_tables.pdf", config=config_ai)

# Access extracted tables
for table in result.tables:
    print(f"Table from page {table['page_number']}:")
    print(table["text"])  # Markdown representation
    # table["df"] contains structured data as Polars DataFrame
```

**See [Table Extraction Guide](table-extraction.md) for:**

- Detailed comparison of AI vs OCR methods
- Performance characteristics and hardware requirements
- Advanced configuration options
- Troubleshooting and optimization tips

### Language Detection

Kreuzberg can automatically detect the language of extracted text using fast-langdetect:

```python
from kreuzberg import extract_file, ExtractionConfig, LanguageDetectionConfig

# Simple automatic language detection
result = await extract_file("multilingual_document.pdf", config=ExtractionConfig(auto_detect_language=True))

# Access detected languages (lowercase ISO 639-1 codes)
if result.detected_languages:
    print(f"Detected languages: {', '.join(result.detected_languages)}")

# Advanced configuration with multilingual detection
lang_config = LanguageDetectionConfig(
    multilingual=True,
    top_k=5,
    low_memory=False,
    cache_dir="/tmp/lang_models",
)

result = await extract_file(
    "multilingual_document.pdf", config=ExtractionConfig(auto_detect_language=True, language_detection_config=lang_config)
)

# Use detected languages for OCR
if result.detected_languages:
    from kreuzberg import TesseractConfig

    result_with_ocr = await extract_file(
        "multilingual_document.pdf",
        config=ExtractionConfig(force_ocr=True, ocr_config=TesseractConfig(language=result.detected_languages[0])),
    )
```

#### Language Detection Configuration Options

- `low_memory` (default: `True`): Use smaller model (~200MB) vs larger, more accurate model
- `multilingual` (default: `False`): Enable detection of multiple languages in mixed text
- `top_k` (default: `3`): Maximum number of languages to return
- `cache_dir`: Custom directory for language model storage
- `allow_fallback` (default: `True`): Fall back to small model if large model fails

The feature requires the `langdetect` dependency:

```shell
pip install "kreuzberg[langdetect]"
```

### Image Extraction

Kreuzberg can extract embedded images from various document formats including PDF, PowerPoint presentations (PPTX), HTML, and Office documents. It also supports running OCR on extracted images to get text content from them.

```python
from kreuzberg import extract_file, ExtractionConfig

# Basic image extraction
result = await extract_file("document.pdf", config=ExtractionConfig(extract_images=True))

# Access extracted images
for i, image in enumerate(result.images):
    print(f"Image {i+1}: {image.format} ({image.dimensions})")
    if image.filename:
        print(f"  Filename: {image.filename}")
    if image.page_number:
        print(f"  Page: {image.page_number}")

    # Save image data to file
    with open(f"extracted_image_{i+1}.{image.format.lower()}", "wb") as f:
        f.write(image.data)
```

#### Image OCR Processing

You can automatically run OCR on extracted images to extract text content:

```python
from kreuzberg import extract_file, ExtractionConfig, ImageOCRConfig

# Extract images and run OCR on them
config = ExtractionConfig(
    extract_images=True,
    image_ocr_config=ImageOCRConfig(
        enabled=True,
        backend="tesseract",  # or "easyocr", "paddleocr"
        min_dimensions=(100, 100),  # Skip small images
        max_dimensions=(5000, 5000),  # Skip very large images
        batch_size=10,  # Process images in parallel
    ),
)

result = await extract_file("presentation.pptx", config=config)

# Access OCR results from images
for ocr_result in result.image_ocr_results:
    image = ocr_result.image
    text_content = ocr_result.ocr_result.content
    confidence = ocr_result.confidence_score

    print(f"Image: {image.filename or 'unnamed'}")
    print(f"OCR Text: {text_content[:100]}...")
    if confidence:
        print(f"Confidence: {confidence:.2f}")
    if ocr_result.skipped_reason:
        print(f"Skipped: {ocr_result.skipped_reason}")
```

#### Image Filtering and Deduplication

Control which images are processed with dimension filtering and deduplication:

```python
from kreuzberg import extract_file, ExtractionConfig, ImageOCRConfig

# Extract only medium-sized images and remove duplicates
config = ExtractionConfig(
    extract_images=True,
    deduplicate_images=True,  # Remove duplicate images by content hash
    image_ocr_config=ImageOCRConfig(
        enabled=True,
        min_dimensions=(200, 200),  # At least 200x200 pixels
        max_dimensions=(3000, 3000),  # At most 3000x3000 pixels
    ),
)

result = await extract_file("document.pdf", config=config)

print(f"Extracted {len(result.images)} unique images")
print(f"OCR processed {len(result.image_ocr_results)} images")
```

#### Advanced Image OCR Configuration

You can use different OCR backends with specific configurations for image processing:

```python
from kreuzberg import extract_file, ExtractionConfig, TesseractConfig, EasyOCRConfig

# Use Tesseract with specific configuration for image OCR
tesseract_config = TesseractConfig(
    language="eng+deu",
    psm=6,  # Uniform block of text
    output_format="text",
)

config = ExtractionConfig(
    extract_images=True,
    ocr_extracted_images=True,
    image_ocr_backend="tesseract",
    image_ocr_config=tesseract_config,
    deduplicate_images=True,
)

result = await extract_file("multilingual_presentation.pptx", config=config)

# Use EasyOCR for better handling of scene text
easyocr_config = EasyOCRConfig(
    language_list=["en", "de"],
    gpu=False,
    confidence_threshold=0.5,
)

config = ExtractionConfig(
    extract_images=True,
    ocr_extracted_images=True,
    image_ocr_backend="easyocr",
    image_ocr_config=easyocr_config,
)

result = await extract_file("document_with_photos.pdf", config=config)
```

#### Supported Image Sources

Image extraction works with these document types:

- **PDF documents**: Extract embedded images and graphics
- **PowerPoint presentations (PPTX)**: Extract slide images, charts, and graphics
- **HTML documents**: Extract inline images and base64-encoded images
- **Microsoft Word documents (DOCX)**: Extract embedded images and charts
- **Email files**: Extract image attachments and inline images

#### Image Extraction Configuration Options

- **`extract_images`** (default: False): Enable image extraction from documents
- **`deduplicate_images`** (default: True): Remove duplicate images based on content hash
- **`image_ocr_config`** (ImageOCRConfig): Configuration for OCR processing of extracted images
    - **`enabled`** (default: False): Run OCR on extracted images
    - **`backend`** (default: None): OCR engine for images ("tesseract", "easyocr", "paddleocr")
    - **`min_dimensions`** (default: (50, 50)): Minimum (width, height) for OCR eligibility
    - **`max_dimensions`** (default: (10000, 10000)): Maximum (width, height) for OCR processing
    - **`batch_size`** (default: 10): Number of images to process in parallel
    - **`allowed_formats`**: Set of image formats to process (jpg, png, gif, etc.)

#### Performance Considerations

Image extraction and OCR can be resource-intensive:

- **Memory usage**: Large images consume significant memory during processing
- **Processing time**: OCR on images takes longer than text extraction
- **Storage**: Extracted image data is included in results, increasing memory usage
- **Filtering**: Use dimension filters to skip very small or very large images
- **Deduplication**: Enable to avoid processing identical images multiple times

For better performance in production:

- Set appropriate dimension limits based on your use case
- Consider using faster OCR backends like Tesseract for batch processing
- Enable deduplication to avoid redundant processing
- Use selective extraction based on document types

### JSON Extraction Configuration

Kreuzberg provides enhanced JSON document processing with schema extraction and customizable field detection:

```python
from kreuzberg import extract_file, ExtractionConfig, JSONExtractionConfig

# Advanced JSON extraction with schema
result = await extract_file(
    "data.json",
    config=ExtractionConfig(
        json_config=JSONExtractionConfig(
            extract_schema=True,  # Extract JSON structure schema
            include_type_info=True,  # Add type annotations to output
            flatten_nested_objects=True,  # Flatten nested objects in output
            custom_text_field_patterns=frozenset({"summary", "abstract"}),  # Additional text fields
            max_depth=10,  # Maximum nesting depth for schema
            array_item_limit=1000,  # Limit array processing for performance
        )
    ),
)

# Access schema and nested attributes
if result.metadata.get("json_schema"):
    print(f"JSON Schema: {result.metadata['json_schema']}")
if result.metadata.get("attributes"):
    print(f"Nested fields: {result.metadata['attributes']}")
```

#### Configuration File Support

Add JSON configuration to your `kreuzberg.toml`:

```toml
[json_config]
extract_schema = true              # Extract JSON structure schema
include_type_info = false          # Add type annotations to output
flatten_nested_objects = true      # Flatten nested objects in output
custom_text_field_patterns = ["summary", "abstract"]  # Additional text fields to extract
max_depth = 10                     # Maximum nesting depth for schema extraction
array_item_limit = 1000           # Limit array processing for performance
```

#### Key Features

- **High Performance**: Uses msgspec for fast JSON parsing, significantly faster than standard library
- **Schema Extraction**: Automatically extracts the structure of your JSON data, useful for understanding complex documents
- **Custom Field Detection**: Configure additional text fields beyond defaults (title, name, description, content, body, text, message)
- **Type Information**: Optionally include data type annotations in extracted content for better understanding
- **Nested Object Control**: Choose between flattened or hierarchical output based on your needs
- **Memory Protection**: Array item limits prevent memory issues with large datasets

### Entity and Keyword Extraction

Kreuzberg can extract named entities and keywords from documents using spaCy for entity recognition and KeyBERT for keyword extraction:

```python
from kreuzberg import extract_file, ExtractionConfig, SpacyEntityExtractionConfig

# Basic entity and keyword extraction
result = await extract_file(
    "document.pdf",
    config=ExtractionConfig(
        extract_entities=True,
        extract_keywords=True,
        keyword_count=10,
    ),
)

# Access extracted entities and keywords
if result.entities:
    for entity in result.entities:
        print(f"{entity.type}: {entity.text} (position {entity.start}-{entity.end})")

if result.keywords:
    for keyword, score in result.keywords:
        print(f"{keyword}: {score:.3f}")
```

#### Entity Extraction with Language Support

spaCy supports entity extraction in multiple languages. You can configure language-specific models:

```python
from kreuzberg import extract_file, ExtractionConfig, SpacyEntityExtractionConfig

# Configure spaCy for specific languages
spacy_config = SpacyEntityExtractionConfig(
    language_models={
        "en": "en_core_web_sm",
        "de": "de_core_news_sm",
        "fr": "fr_core_news_sm",
        "es": "es_core_news_sm",
    },
    model_cache_dir="/tmp/spacy_models",
    fallback_to_multilingual=True,
)

# Extract with language detection to automatically choose the right model
result = await extract_file(
    "multilingual_document.pdf",
    config=ExtractionConfig(
        auto_detect_language=True,
        extract_entities=True,
        spacy_entity_extraction_config=spacy_config,
    ),
)

if result.detected_languages and result.entities:
    print(f"Detected languages: {result.detected_languages}")
    print(f"Extracted {len(result.entities)} entities")
```

#### Custom Entity Patterns

You can define custom entity patterns using regular expressions:

```python
result = await extract_file(
    "invoice.pdf",
    config=ExtractionConfig(
        extract_entities=True,
        custom_entity_patterns={
            "INVOICE_ID": r"INV-\d{4,}",
            "PHONE": r"\+?\d{1,3}[-.\s]?\d{3,4}[-.\s]?\d{3,4}[-.\s]?\d{3,4}",
            "EMAIL": r"[a-zA-Z0-9_.+-]+@[a-zA-Z0-9-]+\.[a-zA-Z0-9-.]+",
        },
    ),
)

for entity in result.entities:
    if entity.type in ["INVOICE_ID", "PHONE", "EMAIL"]:
        print(f"Custom entity - {entity.type}: {entity.text}")
    else:
        print(f"Standard entity - {entity.type}: {entity.text}")
```

#### Supported Entity Types

spaCy automatically detects these standard entity types:

- **PERSON**: People's names
- **ORG**: Organizations, companies, agencies
- **GPE**: Countries, cities, states (Geopolitical entities)
- **MONEY**: Monetary values
- **DATE**: Date expressions
- **TIME**: Time expressions
- **PERCENT**: Percentage values
- **CARDINAL**: Numerals that do not fall under another type

Language-specific models may support additional entity types relevant to that language.

#### spaCy Configuration Options

- `language_models`: Dict mapping language codes to spaCy model names
- `model_cache_dir`: Custom directory for caching spaCy models
- `fallback_to_multilingual`: Whether to use multilingual model (`xx_ent_wiki_sm`) as fallback
- `max_doc_length`: Maximum document length for spaCy processing (default: 1,000,000 characters)
- `batch_size`: Batch size for processing multiple texts (default: 1,000)

#### Installation Requirements

Entity and keyword extraction require additional dependencies:

```shell
# For entity extraction with spaCy
pip install "kreuzberg[entity-extraction]"

# Install specific spaCy language models as needed
python -m spacy download en_core_web_sm    # English
python -m spacy download de_core_news_sm   # German
python -m spacy download fr_core_news_sm   # French
```

Available spaCy models include: `en_core_web_sm`, `de_core_news_sm`, `fr_core_news_sm`, `es_core_news_sm`, `pt_core_news_sm`, `it_core_news_sm`, `nl_core_news_sm`, `zh_core_web_sm`, `ja_core_news_sm`, `ko_core_news_sm`, `ru_core_news_sm`, and many others.

### DPI and Image Processing

Kreuzberg provides intelligent DPI (dots per inch) configuration to optimize document processing quality and performance. This feature automatically handles image scaling for large documents while maintaining OCR quality.

```python
from kreuzberg import extract_file, ExtractionConfig

# Default DPI configuration (optimized for most documents)
result = await extract_file("large_document.pdf")

# Custom DPI configuration for high-quality documents
config = ExtractionConfig(
    target_dpi=200,  # Higher quality for detailed documents
    max_image_dimension=30000,  # Allow larger images
    auto_adjust_dpi=True,  # Automatically scale down if too large
    min_dpi=100,  # Higher minimum for quality
    max_dpi=400,  # Lower maximum to control processing time
)
result = await extract_file("technical_drawing.pdf", config=config)

# Fast processing configuration for large batches
config = ExtractionConfig(
    target_dpi=120,  # Lower DPI for faster processing
    max_image_dimension=15000,  # Smaller maximum size
    auto_adjust_dpi=True,  # Still allow automatic scaling
)
result = await extract_file("large_batch_document.pdf", config=config)
```

#### DPI Configuration Options

- **`target_dpi`** (default: 150): The desired DPI for document processing. Higher values provide better quality but slower processing.

- **`max_image_dimension`** (default: 25000): Maximum pixel dimension (width or height) before automatic scaling kicks in.

- **`auto_adjust_dpi`** (default: True): Automatically reduce DPI for oversized documents to stay within memory and processing limits.

- **`min_dpi`** / **`max_dpi`** (defaults: 72/600): Bounds for automatic DPI adjustment to ensure quality remains within acceptable ranges.

#### When to Adjust DPI Settings

**Increase DPI for:**

- Technical documents with small text or fine details
- Documents that will undergo further image processing
- High-quality archival processing

**Decrease DPI for:**

- Large batch processing where speed is important
- Documents with simple layouts and large text
- Memory-constrained environments

**Use auto-adjustment for:**

- Mixed document types with varying sizes
- Unknown document dimensions
- Production environments processing diverse content

The DPI system prevents "Image too large" errors while maintaining optimal quality-performance balance.

### Batch Processing

```python
from kreuzberg import batch_extract_file, ExtractionConfig

# Process multiple files with the same configuration
file_paths = ["document1.pdf", "document2.docx", "image.jpg"]
config = ExtractionConfig(force_ocr=True)
results = await batch_extract_file(file_paths, config=config)
```

### HTML to Markdown Configuration

Control how HTML content is converted to Markdown:

```python
from kreuzberg import extract_file, ExtractionConfig, HTMLToMarkdownConfig

# Custom HTML to Markdown configuration
html_config = HTMLToMarkdownConfig(
    heading_style="atx",
    strong_em_symbol="_",
    escape_underscores=False,
    wrap=True,
    wrap_width=100,
    list_indent_width=2,  # Discord/Slack compatible spacing
    list_indent_type="spaces",  # Use spaces for indentation
    whitespace_mode="normalized",  # Smart whitespace handling
    br_in_tables=False,  # Use spaces in table cells
    highlight_style="double-equal",  # ==highlighted== text style
    newline_style="spaces",  # Line break style
    preprocess_html=True,  # Clean HTML before conversion
    preprocessing_preset="standard",  # HTML cleaning level
)

result = await extract_file(
    "document.html",
    config=ExtractionConfig(html_to_markdown_config=html_config),
)
```

Available heading styles:

- `"underlined"`: Classic Markdown with underlines for h1/h2
- `"atx"`: Hash-based headers (e.g., `# Header`)
- `"atx_closed"`: Hash-based with closing hashes

### Synchronous API

```python
from kreuzberg import extract_file_sync, ExtractionConfig, TesseractConfig

# Synchronous extraction with configuration
result = extract_file_sync("document.pdf", config=ExtractionConfig(ocr_config=TesseractConfig(language="eng")))
```

## Using Custom Extractors

You can register custom extractors to handle specific file formats:

```python
from kreuzberg import ExtractorRegistry, extract_file, ExtractionConfig
from my_module import CustomExtractor

# Register a custom extractor
ExtractorRegistry.add_extractor(CustomExtractor)

# Now extraction functions will use your custom extractor for supported MIME types
result = await extract_file("custom_document.xyz")

# Later, remove the extractor if needed
ExtractorRegistry.remove_extractor(CustomExtractor)
```

See the [Custom Extractors](../advanced/custom-extractors.md) guide for more details on creating and registering custom extractors.

## OCR Best Practices

When configuring OCR for your documents, consider these best practices:

1. **Language Selection**: Choose the appropriate language model for your documents. Using the wrong language model can significantly reduce OCR accuracy.

1. **Page Segmentation Mode**: Select the appropriate PSM based on your document layout:

    - Use `PSMMode.AUTO` for general documents with mixed content
    - Use `PSMMode.SINGLE_BLOCK` for documents with a single column of text
    - Use `PSMMode.SINGLE_LINE` for receipts or single-line text
    - Use `PSMMode.SINGLE_WORD` or `PSMMode.SINGLE_CHAR` for specialized cases

1. **OCR Engine Selection**: Choose the appropriate OCR engine based on your needs:

    - Tesseract: Good general-purpose OCR with support for many languages
    - EasyOCR: Better for some non-Latin scripts and natural scene text
    - PaddleOCR: Excellent for Chinese and other Asian languages

1. **Preprocessing**: For better OCR results, consider using validation and post-processing hooks to clean up the extracted text.
