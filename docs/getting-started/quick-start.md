# Quick Start

Get started with Kreuzberg for text extraction from documents and images.

## Basic Usage

Kreuzberg provides both asynchronous and synchronous APIs for text extraction.

### Async API (Recommended)

```python
import asyncio
from kreuzberg import extract_file

async def main():
    # Extract text from a PDF file
    result = await extract_file("document.pdf")
    print(result.content)

    # The result also contains metadata
    print(f"Mime type: {result.mime_type}")
    print(f"Extraction method: {result.extraction_method}")

asyncio.run(main())
```

### Synchronous API

```python
from kreuzberg import extract_file_sync

# Extract text from a PDF file
result = extract_file_sync("document.pdf")
print(result.content)
```

## OCR Configuration

Kreuzberg supports OCR for images and scanned PDFs with configurable language and page segmentation mode:

```python
from kreuzberg import extract_file, ExtractionConfig, TesseractConfig, PSMMode

async def main():
    # Extract text from an image with German language model
    result = await extract_file(
        "german_document.jpg",
        config=ExtractionConfig(
            ocr=TesseractConfig(
                language="deu",  # German language model
                psm=PSMMode.SINGLE_BLOCK,  # Treat as a single text block
            )
        ),
    )
    print(result.content)

asyncio.run(main())
```

## Batch Processing

Process multiple files concurrently:

```python
from pathlib import Path
from kreuzberg import batch_extract_file

async def process_documents():
    file_paths = [Path("document1.pdf"), Path("document2.docx"), Path("image.jpg")]

    # Process all files concurrently
    results = await batch_extract_file(file_paths)

    # Results are returned in the same order as inputs
    for path, result in zip(file_paths, results):
        print(f"File: {path}")
        print(f"Content: {result.content[:100]}...")  # First 100 chars
        print(f"Mime type: {result.mime_type}")
        print(f"Method: {result.extraction_method}")
        print("---")

asyncio.run(process_documents())
```

## Error Handling

Kreuzberg provides [specific exceptions](../api-reference/exceptions.md) for different error cases:

```python
from kreuzberg import extract_file
from kreuzberg import KreuzbergError, MissingDependencyError, OCRError, ParsingError

async def safe_extract(path):
    try:
        result = await extract_file(path)
        return result.content
    except ParsingError:
        print(f"Unsupported or invalid file format: {path}")
    except MissingDependencyError as e:
        print(f"Missing dependency: {e}")
    except OCRError as e:
        print(f"OCR processing failed: {e}")
    except KreuzbergError as e:
        print(f"Extraction failed: {e}")
    return None
```

## Next Steps

- Check the [OCR Configuration](../user-guide/ocr-configuration.md) guide for detailed OCR options
- See [Supported Formats](../user-guide/supported-formats.md) for all file types Kreuzberg can process
- Explore the [API Reference](../api-reference/extraction-functions.md) for detailed function documentation
