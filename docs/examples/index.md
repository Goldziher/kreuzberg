# Kreuzberg Examples

This section provides practical examples of using Kreuzberg in various scenarios. These examples show real-world use cases and can serve as a starting point for your own implementations.

## Available Examples

- [Extraction Examples](extraction-examples.md) - Various text extraction patterns and configurations

## Code Samples

### Basic Text Extraction

```python
import asyncio
from kreuzberg import extract_file

async def main():
    result = await extract_file("document.pdf")
    print(f"Text content: {result.content[:200]}...")  # First 200 chars
    print(f"Document type: {result.mime_type}")

    if "title" in result.metadata:
        print(f"Title: {result.metadata['title']}")

asyncio.run(main())
```

### Multi-language OCR

```python
from kreuzberg import extract_file, ExtractionConfig, TesseractConfig

async def extract_multilingual():
    # Configure for English and German text
    config = ExtractionConfig(ocr=TesseractConfig(language="eng+deu"))

    result = await extract_file("multilingual.pdf", config=config)
    return result.content

# For Chinese text using PaddleOCR
from kreuzberg import PaddleOCRConfig

async def extract_chinese():
    config = ExtractionConfig(ocr=PaddleOCRConfig(language="ch"))

    result = await extract_file("chinese_document.jpg", config=config)
    return result.content
```
