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

## Automatic Language Detection

Kreuzberg can automatically detect the language of your documents and configure OCR accordingly:

```python
from kreuzberg import extract_file, ExtractionConfig

async def extract_with_language_detection():
    # Enable automatic language detection
    config = ExtractionConfig(auto_detect_language=True)
    
    # Extract from a German document
    result = await extract_file("german_document.pdf", config=config)
    print(f"Detected languages: {result.detected_languages}")
    print(f"Content: {result.content[:100]}...")
    
    # Extract from a multilingual document
    result = await extract_file("multilingual.pdf", config=config)
    print(f"Detected languages: {result.detected_languages}")  # e.g., ['en', 'de']
    print(f"Content: {result.content[:100]}...")

async def extract_with_fallback():
    # Combine automatic detection with manual configuration
    config = ExtractionConfig(
        auto_detect_language=True,
        ocr_config=TesseractConfig(language="eng")  # Fallback to English
    )
    
    result = await extract_file("document.pdf", config=config)
    if result.detected_languages:
        print(f"Using detected languages: {result.detected_languages}")
    else:
        print("Using fallback language configuration")
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

## Table Extraction

```python
from kreuzberg import extract_file, ExtractionConfig, GMFTConfig

async def extract_tables_from_pdf():
    # Enable table extraction with default settings
    result = await extract_file("document_with_tables.pdf", config=ExtractionConfig(extract_tables=True))

    # Process extracted tables
    print(f"Found {len(result.tables)} tables")
    for i, table in enumerate(result.tables):
        print(f"Table {i+1} on page {table.page_number}:")
        print(table.text)  # Markdown formatted table

        # Work with the pandas DataFrame
        df = table.df
        print(f"Table shape: {df.shape}")

        # The cropped table image is also available
        # table.cropped_image.save(f"table_{i+1}.png")

    # With custom GMFT configuration
    custom_config = ExtractionConfig(
        extract_tables=True,
        gmft_config=GMFTConfig(
            detector_base_threshold=0.85,  # Min confidence for table detection
            enable_multi_header=True,  # Support multi-level headers
            semantic_spanning_cells=True,  # Handle spanning cells
            semantic_hierarchical_left_fill="deep",  # Handle hierarchical headers
        ),
    )

    result = await extract_file("complex_tables.pdf", config=custom_config)
    # Process tables...
```

## Batch Processing

```python
from kreuzberg import batch_extract_file, ExtractionConfig

async def process_documents():
    file_paths = ["document1.pdf", "document2.docx", "image.jpg"]
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
