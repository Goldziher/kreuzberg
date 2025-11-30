# Basic Usage

Kreuzberg provides a unified API for document intelligence operations, supporting both synchronous and asynchronous processing patterns.

## Core Functions

Kreuzberg exports the following main functions:

### Single Item Processing

- [`extract_file()`](../api-reference/extraction-functions.md#extract_file): Async function to extract text from a file (accepts string path or `pathlib.Path`)
- [`extract_bytes()`](../api-reference/extraction-functions.md#extract_bytes): Async function to extract text from bytes (accepts a byte string)
- [`extract_file_sync()`](../api-reference/extraction-functions.md#extract_file_sync): Synchronous version of `extract_file()`
- [`extract_bytes_sync()`](../api-reference/extraction-functions.md#extract_bytes_sync): Synchronous version of `extract_bytes()`

### Batch Processing

- [`batch_extract_file()`](../api-reference/extraction-functions.md#batch_extract_file): Async function to extract text from multiple files concurrently
- [`batch_extract_bytes()`](../api-reference/extraction-functions.md#batch_extract_bytes): Async function to extract text from multiple byte contents concurrently
- [`batch_extract_file_sync()`](../api-reference/extraction-functions.md#batch_extract_file_sync): Synchronous version of `batch_extract_file()`
- [`batch_extract_bytes_sync()`](../api-reference/extraction-functions.md#batch_extract_bytes_sync): Synchronous version of `batch_extract_bytes()`

## Async Examples

### Extract Text from a File

```python
import asyncio
from kreuzberg import extract_file

async def main():
    result = await extract_file("document.pdf")
    print(result.content)
    print(f"MIME type: {result.mime_type}")
    print(f"Metadata: {result.metadata}")

asyncio.run(main())
```

### Process Multiple Files Concurrently

```python
import asyncio
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
        print(f"MIME type: {result.mime_type}")
        print("---")

asyncio.run(process_documents())
```

## Synchronous Examples

### Extract Text from a File

```python
from kreuzberg import extract_file_sync

result = extract_file_sync("document.pdf")
print(result.content)
```

### Process Multiple Files

```python
from kreuzberg import batch_extract_file_sync

file_paths = ["document1.pdf", "document2.docx", "image.jpg"]
results = batch_extract_file_sync(file_paths)

for path, result in zip(file_paths, results):
    print(f"File: {path}")
    print(f"Content: {result.content[:100]}...")
```

## Working with Byte Content

If you already have the file content in memory, you can use the bytes extraction functions:

```python
import asyncio
from kreuzberg import extract_bytes

async def extract_from_memory():
    with open("document.pdf", "rb") as f:
        content = f.read()

    result = await extract_bytes(content, mime_type="application/pdf")
    print(result.content)

asyncio.run(extract_from_memory())
```

## Extraction Result

All extraction functions return an [`ExtractionResult`](../api-reference/types.md#extractionresult) object containing:

- `content`: Extracted text
- `mime_type`: Document MIME type
- `metadata`: Document metadata (see [Metadata Extraction](metadata-extraction.md))

```python
from kreuzberg import extract_file, ExtractionResult  # Import types directly from kreuzberg

async def show_metadata():
    result: ExtractionResult = await extract_file("document.pdf")

    # Access the content
    print(result.content)

    # Access metadata (if available)
    if "title" in result.metadata:
        print(f"Title: {result.metadata['title']}")

    if "authors" in result.metadata:
        print(f"Authors: {', '.join(result.metadata['authors'])}")

    if "created_at" in result.metadata:
        print(f"Created: {result.metadata['created_at']}")

asyncio.run(show_metadata())
```

## Document Classification

Kreuzberg can automatically classify documents into categories (contracts, forms, invoices, receipts, reports):

```python
import asyncio
from kreuzberg import extract_file, ExtractionConfig

async def classify_document():
    config = ExtractionConfig(
        auto_detect_document_type=True,
        document_classification_mode="text",  # or "vision" for better accuracy
        type_confidence_threshold=0.5,
    )

    result = await extract_file("invoice.pdf", config=config)

    # Access classification results
    if result.document_type:
        print(f"Document type: {result.document_type}")
        print(f"Confidence: {result.type_confidence:.2%}")

    # The extracted content is still available
    print(f"Content: {result.content[:200]}...")

asyncio.run(classify_document())
```
