# Extraction Examples

This page provides practical examples of using Kreuzberg for text extraction in various scenarios.

## Basic Extraction

```python
import asyncio
from kreuzberg import extract_file

async def main():
    # Extract text from a PDF file
    result = await extract_file("document.pdf")
    print(result.content)

    # Access metadata
    if result.metadata.get("title"):
        print(f"Document title: {result.metadata['title']}")

asyncio.run(main())
```

## OCR Configuration

Kreuzberg provides options to configure OCR for different languages and document layouts:

```python
from kreuzberg import extract_file, TesseractConfig, PSMMode, ExtractionConfig

async def extract_with_ocr():
    # Extract from a German document
    result = await extract_file(
        "german_document.pdf",
        config=ExtractionConfig(
            force_ocr=True,
            ocr_config=TesseractConfig(
                language="deu", psm=PSMMode.SINGLE_BLOCK  # German language  # Treat as a single text block
            ),
        ),
    )
    print(result.content)

    # Extract from a multilingual document
    result = await extract_file(
        "multilingual.pdf",
        config=ExtractionConfig(
            force_ocr=True,
            ocr_config=TesseractConfig(
                language="eng+deu", psm=PSMMode.AUTO  # English primary, German secondary  # Automatic page segmentation
            ),
        ),
    )
    print(result.content)
```

## Alternative OCR Backends

Kreuzberg supports multiple OCR backends:

```python
from kreuzberg import extract_file, ExtractionConfig, EasyOCRConfig, PaddleOCRConfig

async def extract_with_different_backends():
    # Using EasyOCR
    result = await extract_file(
        "document.jpg", config=ExtractionConfig(ocr_backend="easyocr", ocr_config=EasyOCRConfig(language_list=["en", "de"]))
    )
    print(f"EasyOCR result: {result.content[:100]}...")

    # Using PaddleOCR
    result = await extract_file(
        "chinese_document.jpg",
        config=ExtractionConfig(ocr_backend="paddleocr", ocr_config=PaddleOCRConfig(language="ch")),  # Chinese
    )
    print(f"PaddleOCR result: {result.content[:100]}...")

    # Disable OCR completely
    result = await extract_file("searchable_pdf.pdf", config=ExtractionConfig(ocr_backend=None))
    print(f"No OCR result: {result.content[:100]}...")
```

## Language Detection

```python
from kreuzberg import extract_file, ExtractionConfig, LanguageDetectionConfig

async def detect_document_language():
    # Simple automatic language detection
    result = await extract_file("document.pdf", config=ExtractionConfig(auto_detect_language=True))

    # Access detected languages
    if result.detected_languages:
        print(f"Detected languages: {', '.join(result.detected_languages)}")
        # Example output: "Detected languages: en, de, fr"

async def detect_multilingual_document():
    # Advanced multilingual detection with custom configuration
    lang_config = LanguageDetectionConfig(
        multilingual=True,  # Detect multiple languages in mixed text
        top_k=5,  # Return top 5 languages
        low_memory=False,  # Use high accuracy mode
    )

    result = await extract_file(
        "multilingual_document.pdf", config=ExtractionConfig(auto_detect_language=True, language_detection_config=lang_config)
    )

    if result.detected_languages:
        print(f"Detected languages: {result.detected_languages}")

        # Use detected languages for OCR
        from kreuzberg import TesseractConfig

        # Create language string for Tesseract (e.g., "eng+deu+fra")
        tesseract_langs = "+".join(result.detected_languages[:3])

        result_with_ocr = await extract_file(
            "multilingual_document.pdf",
            config=ExtractionConfig(force_ocr=True, ocr_config=TesseractConfig(language=tesseract_langs)),
        )
```

## Table Extraction

Kreuzberg offers multiple methods for extracting tables from documents. See the [Table Extraction Guide](../user-guide/table-extraction.md) for detailed comparison.

```python
from kreuzberg import extract_file, ExtractionConfig, GMFTConfig

async def extract_tables_examples():
    # Vision-based table extraction (best for complex tables)
    ai_config = ExtractionConfig(
        extract_tables=True,
        gmft_config=GMFTConfig(
            detection_threshold=0.7,
            structure_threshold=0.5,
        ),
    )

    result = await extract_file("complex_tables.pdf", config=ai_config)

    # OCR-based table extraction (lightweight, good for simple tables)
    ocr_config = ExtractionConfig(extract_tables_from_ocr=True)

    result_ocr = await extract_file("simple_tables.pdf", config=ocr_config)

    # Combined approach (maximum table coverage)
    combined_config = ExtractionConfig(
        extract_tables=True,  # Vision method
        extract_tables_from_ocr=True,  # OCR method
    )

    result_combined = await extract_file("mixed_tables.pdf", config=combined_config)

    # Process extracted tables
    for i, table in enumerate(result.tables):
        print(f"Table {i+1} on page {table['page_number']}:")
        print(table["text"])  # Markdown representation

        # Access structured data
        df = table["df"]  # Polars DataFrame
        print(f"Table dimensions: {df.shape[0]} rows × {df.shape[1]} columns")

        # Save table data in different formats
        df.write_csv(f"table_{i+1}.csv")

        # Save table image (available with vision method)
        if "cropped_image" in table:
            table["cropped_image"].save(f"table_{i+1}.png")

# Compare methods
async def compare_table_methods():
    """Example showing when to use different table extraction methods."""

    # For scientific papers with complex tables - use vision method
    scientific_config = ExtractionConfig(
        extract_tables=True,
        gmft_config=GMFTConfig(
            detection_threshold=0.6,  # Lower threshold for academic tables
        ),
    )

    # For simple forms and basic tables - use OCR method
    forms_config = ExtractionConfig(extract_tables_from_ocr=True)

    # For financial reports - use higher precision settings
    financial_config = ExtractionConfig(
        extract_tables=True,
        gmft_config=GMFTConfig(
            detection_threshold=0.8,  # Higher threshold for clean financial docs
        ),
    )

    print("See Table Extraction Guide for detailed method comparison")
```

## OCR Output Formats and Table Extraction

### Choosing the Right Output Format

Kreuzberg's Tesseract backend supports multiple output formats, each optimized for different use cases.

#### Fast Plain Text Extraction

Use the `text` format for the fastest extraction when you don't need formatting:

```python
from kreuzberg import extract_file, ExtractionConfig, TesseractConfig

async def extract_plain_text():
    result = await extract_file("document.jpg", config=ExtractionConfig(ocr_config=TesseractConfig(output_format="text")))
    print(result.content)
```

#### Default Markdown with Structure

The default `markdown` format preserves document structure:

```python
from kreuzberg import extract_file

async def extract_with_markdown():
    # Markdown is the default format
    result = await extract_file("document.jpg")
    print(result.content)  # Structured markdown output
```

#### Extract Tables from Scanned Documents

Use TSV format with table detection to extract tables from images:

```python
from kreuzberg import extract_file, ExtractionConfig, TesseractConfig

async def extract_tables():
    result = await extract_file(
        "scanned_table.png",
        config=ExtractionConfig(
            ocr_config=TesseractConfig(
                output_format="tsv",
                enable_table_detection=True,
                table_column_threshold=20,  # Adjust for column spacing
                table_row_threshold_ratio=0.5,  # Adjust for row spacing
            )
        ),
    )

    # Access extracted tables
    for table in result.tables:
        print("Extracted table in markdown format:")
        print(table["text"])
        print(f"Page number: {table['page_number']}")
```

#### Get Word Positions with hOCR

Use hOCR format to access detailed position information:

```python
from kreuzberg import extract_file, ExtractionConfig, TesseractConfig

async def extract_with_positions():
    result = await extract_file("document.jpg", config=ExtractionConfig(ocr_config=TesseractConfig(output_format="hocr")))
    # result.content contains HTML with position data
    print(result.content[:500])  # hOCR HTML output
```

### Processing Scanned Invoices

Complete example for extracting data from scanned invoices:

```python
from kreuzberg import extract_file, ExtractionConfig, TesseractConfig, PSMMode

async def process_invoice():
    # Configure for invoice processing
    config = ExtractionConfig(
        ocr_config=TesseractConfig(
            output_format="tsv",
            enable_table_detection=True,
            table_column_threshold=25,  # Adjust for invoice layout
            psm=PSMMode.SPARSE_TEXT,  # Good for forms and invoices
            language="eng",
        )
    )

    result = await extract_file("invoice_scan.pdf", config=config)

    # Get the main text content
    print("Invoice text:")
    print(result.content)

    # Extract tables (line items)
    if result.tables:
        print("\nInvoice line items:")
        for table in result.tables:
            print(table["text"])
```

## Image Extraction

Kreuzberg can extract embedded images from various document formats and optionally run OCR on them to extract text content.

### Basic Image Extraction

```python
from kreuzberg import extract_file, ExtractionConfig
import base64
from pathlib import Path

async def extract_images_from_pdf():
    # Extract embedded images from a PDF document
    result = await extract_file("document_with_images.pdf", config=ExtractionConfig(extract_images=True))

    print(f"Document content: {result.content[:100]}...")
    print(f"Found {len(result.images)} images")

    # Save extracted images to files
    for i, image in enumerate(result.images):
        filename = image.filename or f"image_{i+1}.{image.format.lower()}"
        filepath = Path("extracted_images") / filename
        filepath.parent.mkdir(exist_ok=True)

        # Write image data to file
        filepath.write_bytes(image.data)

        print(f"Saved image: {filename}")
        print(f"  Format: {image.format}")
        print(f"  Dimensions: {image.dimensions}")
        if image.page_number:
            print(f"  Page: {image.page_number}")
        if image.description:
            print(f"  Description: {image.description}")
```

### Image OCR Processing

Extract text content from images using OCR:

```python
from kreuzberg import extract_file, ExtractionConfig, ImageOCRConfig

async def extract_and_ocr_images():
    # Extract images and run OCR on them
    config = ExtractionConfig(
        extract_images=True,
        deduplicate_images=True,  # Remove duplicate images
        image_ocr_config=ImageOCRConfig(
            enabled=True,
            backend="tesseract",
            # Only process reasonably sized images
            min_dimensions=(100, 100),
            max_dimensions=(3000, 3000),
        ),
    )

    result = await extract_file("presentation.pptx", config=config)

    print(f"Main content: {len(result.content)} characters")
    print(f"Extracted {len(result.images)} unique images")
    print(f"OCR processed {len(result.image_ocr_results)} images")

    # Process OCR results
    for i, ocr_result in enumerate(result.image_ocr_results):
        image = ocr_result.image
        text = ocr_result.ocr_result.content
        confidence = ocr_result.confidence_score

        print(f"\nImage {i+1}: {image.filename or 'unnamed'}")
        print(f"  Dimensions: {image.dimensions}")
        print(f"  OCR Confidence: {confidence:.2f}" if confidence else "  No confidence score")

        if ocr_result.skipped_reason:
            print(f"  Skipped: {ocr_result.skipped_reason}")
        else:
            print(f"  Extracted text: {text[:100]}...")
```

### Advanced Image OCR Configuration

Use different OCR backends and configurations for optimal results:

```python
from kreuzberg import extract_file, ExtractionConfig, TesseractConfig, EasyOCRConfig, PSMMode

async def advanced_image_ocr():
    # Tesseract with multilingual support for technical documents
    tesseract_config = TesseractConfig(
        language="eng+deu",  # English and German
        psm=PSMMode.SINGLE_BLOCK,  # Treat each image as single text block
        output_format="text",
        tessedit_char_whitelist="0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz .-,",
    )

    config = ExtractionConfig(
        extract_images=True,
        image_ocr_config=ImageOCRConfig(
            enabled=True,
            backend="tesseract",
            backend_config=tesseract_config,
            min_dimensions=(150, 50),  # Allow narrow images like table headers
            max_dimensions=(4000, 4000),
        ),
    )

    result = await extract_file("technical_manual.pdf", config=config)

    # EasyOCR for natural scene text and photos
    easyocr_config = EasyOCRConfig(
        language_list=["en"],
        gpu=False,  # Use CPU processing
        confidence_threshold=0.4,  # Lower threshold for challenging images
    )

    config = ExtractionConfig(
        extract_images=True,
        image_ocr_config=ImageOCRConfig(
            enabled=True,
            backend="easyocr",
            backend_config=easyocr_config,
        ),
    )

    result = await extract_file("document_with_photos.pdf", config=config)

    # Filter OCR results by confidence
    high_confidence_results = [
        ocr_result for ocr_result in result.image_ocr_results if ocr_result.confidence_score and ocr_result.confidence_score > 0.7
    ]

    print(f"High-confidence OCR results: {len(high_confidence_results)}")
```

### Processing Different Document Types

Image extraction works with various document formats:

```python
from kreuzberg import extract_file, ExtractionConfig, ImageOCRConfig

async def extract_from_various_formats():
    config = ExtractionConfig(
        extract_images=True,
        image_ocr_config=ImageOCRConfig(
            enabled=True,
            backend="tesseract",
        ),
    )

    # PDF documents - embedded images and graphics
    pdf_result = await extract_file("report.pdf", config=config)
    print(f"PDF: {len(pdf_result.images)} images extracted")

    # PowerPoint presentations - slide images and shapes
    pptx_result = await extract_file("presentation.pptx", config=config)
    print(f"PPTX: {len(pptx_result.images)} images extracted")

    # HTML documents - inline and base64 images
    html_result = await extract_file("webpage.html", config=config)
    print(f"HTML: {len(html_result.images)} images extracted")

    # Email messages - attachments and inline images
    email_result = await extract_file("message.eml", config=config)
    print(f"Email: {len(email_result.images)} images extracted")

    # Word documents - embedded images and charts
    docx_result = await extract_file("document.docx", config=config)
    print(f"DOCX: {len(docx_result.images)} images extracted")
```

### Image Processing Performance Optimization

Control performance and resource usage:

```python
from kreuzberg import extract_file, ExtractionConfig, ImageOCRConfig

async def optimized_image_processing():
    # Fast processing for large batches
    fast_config = ExtractionConfig(
        extract_images=True,
        deduplicate_images=True,  # Remove duplicates
        # image_ocr_config not set, so OCR is disabled
    )

    # Quality processing for important documents
    quality_config = ExtractionConfig(
        extract_images=True,
        deduplicate_images=True,
        image_ocr_config=ImageOCRConfig(
            enabled=True,
            backend="tesseract",
            min_dimensions=(50, 50),  # Process smaller images
            max_dimensions=(5000, 5000),  # Allow larger images
        ),
    )

    # Selective processing based on image size
    selective_config = ExtractionConfig(
        extract_images=True,
        deduplicate_images=True,
        image_ocr_config=ImageOCRConfig(
            enabled=True,
            backend="tesseract",
            min_dimensions=(300, 100),  # Good for charts and tables
            max_dimensions=(3000, 3000),
        ),
    )

    # Process with different configs
    for config_name, config in [("fast", fast_config), ("quality", quality_config), ("selective", selective_config)]:
        result = await extract_file("large_document.pdf", config=config)
        print(f"{config_name.title()} mode: {len(result.images)} images, " f"{len(result.image_ocr_results)} OCR results")
```

### Combining Image Extraction with Other Features

Use image extraction alongside other Kreuzberg features:

```python
from kreuzberg import extract_file, ExtractionConfig, ImageOCRConfig

async def comprehensive_extraction():
    config = ExtractionConfig(
        # Text extraction
        chunk_content=True,
        max_chars=1000,
        # Image extraction
        extract_images=True,
        image_ocr_config=ImageOCRConfig(
            enabled=True,
            backend="tesseract",
        ),
        # Table extraction
        extract_tables=True,
        # Entity and keyword extraction
        extract_entities=True,
        extract_keywords=True,
        keyword_count=10,
        # Language detection
        auto_detect_language=True,
    )

    result = await extract_file("comprehensive_document.pdf", config=config)

    print("=== Comprehensive Extraction Results ===")
    print(f"Main content: {len(result.content)} characters")
    print(f"Content chunks: {len(result.chunks)}")
    print(f"Extracted images: {len(result.images)}")
    print(f"Image OCR results: {len(result.image_ocr_results)}")
    print(f"Tables: {len(result.tables)}")
    print(f"Detected languages: {result.detected_languages}")
    print(f"Keywords: {len(result.keywords) if result.keywords else 0}")
    print(f"Entities: {len(result.entities) if result.entities else 0}")

    # Combine text from main content and image OCR
    all_text = result.content
    for ocr_result in result.image_ocr_results:
        if ocr_result.ocr_result.content.strip():
            all_text += "\n\nFrom image OCR:\n" + ocr_result.ocr_result.content

    print(f"Total text (including OCR): {len(all_text)} characters")
```

## JSON and Structured Data Extraction

### Basic JSON Extraction

```python
from kreuzberg import extract_file_sync

# Simple JSON extraction
result = extract_file_sync("data.json")
print(result.content)

# Metadata includes detected text fields
print(f"Title: {result.metadata.get('title')}")
print(f"Description: {result.metadata.get('description')}")
```

### Advanced JSON with Schema Extraction

```python
from kreuzberg import extract_file_sync, ExtractionConfig, JSONExtractionConfig

# Configure advanced JSON extraction
json_config = JSONExtractionConfig(
    extract_schema=True,  # Extract JSON structure
    custom_text_field_patterns=frozenset({"summary", "abstract"}),  # Custom fields
    include_type_info=True,  # Add type annotations
    flatten_nested_objects=True,  # Flatten nested structures
    max_depth=5,  # Limit schema depth
    array_item_limit=100,  # Limit array processing
)

config = ExtractionConfig(json_config=json_config)
result = extract_file_sync("complex.json", config=config)

# Access schema information
if "json_schema" in result.metadata:
    schema = result.metadata["json_schema"]
    print(f"Root type: {schema['type']}")
    print(f"Properties: {list(schema.get('properties', {}).keys())}")

# Access nested attributes with dotted notation
if "attributes" in result.metadata:
    attrs = result.metadata["attributes"]
    # Nested fields like {"info": {"title": "Example"}} become "info.title"
    print(f"Nested title: {attrs.get('info.title')}")
```

### YAML and TOML Processing

```python
from kreuzberg import extract_file_sync

# YAML extraction (similar to JSON)
yaml_result = extract_file_sync("config.yaml")
print(yaml_result.content)

# TOML extraction
toml_result = extract_file_sync("pyproject.toml")
print(toml_result.content)

# Both formats support the same metadata extraction as JSON
print(f"Package name: {toml_result.metadata.get('name')}")
```

### Working with API Responses

```python
import httpx
from kreuzberg import extract_bytes_sync, ExtractionConfig, JSONExtractionConfig

# Fetch JSON from API
response = httpx.get("https://api.example.com/data")

# Extract with schema
config = ExtractionConfig(json_config=JSONExtractionConfig(extract_schema=True))

result = extract_bytes_sync(response.content, mime_type="application/json", config=config)

print(f"API Response: {result.content}")
print(f"Schema: {result.metadata.get('json_schema')}")
```

## Batch Processing

```python
from kreuzberg import batch_extract_file, ExtractionConfig

async def process_documents():
    file_paths = ["document1.pdf", "document2.docx", "data.json", "image.jpg"]
    config = ExtractionConfig()  # Optional: configure extraction options
    results = await batch_extract_file(file_paths, config=config)

    for path, result in zip(file_paths, results):
        print(f"File: {path}")
        print(f"Content: {result.content[:100]}...")
```

## Working with Bytes

```python
from kreuzberg import extract_bytes, ExtractionConfig

async def process_upload(file_content: bytes, mime_type: str):
    # Extract text from uploaded file content
    config = ExtractionConfig()  # Optional: configure extraction options
    result = await extract_bytes(file_content, mime_type=mime_type, config=config)
    print(f"Content: {result.content[:100]}...")

    # Access metadata
    if result.metadata:
        for key, value in result.metadata.items():
            print(f"{key}: {value}")
```

## Keywords

Kreuzberg supports keywords and regex extraction as follows:

```python
from kreuzberg import ExtractionConfig, extract_file

async def extract_keywords():
    config = ExtractionConfig(
        extract_keywords=True,
        keyword_count=5,  # defaults to 10 if not set
    )
    result = await extract_file(
        "document.pdf",
        config=config,
    )
    print(f"Keywords: {result.keywords}")
```

## Entity and Keyword Extraction

Kreuzberg can extract named entities using spaCy and keywords using KeyBERT. It automatically detects entities like people, organizations, locations, and more, plus supports custom regex patterns:

```python
from kreuzberg import ExtractionConfig, extract_file, SpacyEntityExtractionConfig

async def extract_entities_and_keywords():
    # Basic extraction
    config = ExtractionConfig(
        extract_entities=True,
        extract_keywords=True,
        keyword_count=5,
        custom_entity_patterns={
            "INVOICE_ID": r"INV-\d+",
            "EMAIL": r"[a-zA-Z0-9_.+-]+@[a-zA-Z0-9-]+\.[a-zA-Z0-9-.]+",
        },
    )
    result = await extract_file("document.pdf", config=config)

    # Print extracted entities
    if result.entities:
        for entity in result.entities:
            print(f"{entity.type}: {entity.text}")

    # Print extracted keywords
    if result.keywords:
        for keyword, score in result.keywords:
            print(f"Keyword: {keyword} (score: {score:.3f})")

async def extract_multilingual_entities():
    # Configure spaCy for multiple languages
    spacy_config = SpacyEntityExtractionConfig(
        language_models={
            "en": "en_core_web_sm",
            "de": "de_core_news_sm",
            "fr": "fr_core_news_sm",
        },
        fallback_to_multilingual=True,
    )

    config = ExtractionConfig(
        auto_detect_language=True,  # Automatically detect document languages
        extract_entities=True,
        spacy_entity_extraction_config=spacy_config,
    )

    result = await extract_file("multilingual_document.pdf", config=config)

    if result.detected_languages:
        print(f"Detected languages: {result.detected_languages}")

    if result.entities:
        print(f"Extracted {len(result.entities)} entities")
        for entity in result.entities:
            print(f"  {entity.type}: {entity.text}")
```

## Synchronous API

For cases where async isn't needed or available:

```python
from kreuzberg import extract_file_sync, batch_extract_file_sync, ExtractionConfig

# Configuration for extraction
config = ExtractionConfig()  # Optional: configure extraction options

# Single file extraction
result = extract_file_sync("document.pdf", config=config)
print(result.content)

# Batch processing
file_paths = ["document1.pdf", "document2.docx", "image.jpg"]
results = batch_extract_file_sync(file_paths, config=config)
for path, result in zip(file_paths, results):
    print(f"File: {path}")
    print(f"Content: {result.content[:100]}...")
```

## Error Handling

```python
from kreuzberg import extract_file, ExtractionConfig
from kreuzberg import KreuzbergError, MissingDependencyError, OCRError

async def safe_extract(path):
    try:
        config = ExtractionConfig()  # Optional: configure extraction options
        result = await extract_file(path, config=config)
        return result.content
    except MissingDependencyError as e:
        print(f"Missing dependency: {e}")
        print("Please install the required dependencies.")
    except OCRError as e:
        print(f"OCR processing failed: {e}")
    except KreuzbergError as e:
        print(f"Extraction error: {e}")
    except Exception as e:
        print(f"Unexpected error: {e}")

    return None
```
