```python title="Python"
from kreuzberg import extract_file_sync, extract_bytes_sync, ExtractionConfig
from kreuzberg import (
    KreuzbergError,
    ParsingError,
    OCRError,
    ValidationError,
)

try:
    result = extract_file_sync("document.pdf")
    print(f"Extracted {len(result.content)} characters")
except ParsingError as e:
    print(f"Failed to parse document: {e}")
except OCRError as e:
    print(f"OCR processing failed: {e}")
except KreuzbergError as e:
    print(f"Extraction error: {e}")

try:
    config: ExtractionConfig = ExtractionConfig()
    pdf_bytes: bytes = b"%PDF-1.4\n"
    result = extract_bytes_sync(pdf_bytes, "application/pdf", config)
    print(f"Extracted: {result.content[:100]}")
except ValidationError as e:
    print(f"Invalid configuration: {e}")
except OCRError as e:
    print(f"OCR failed: {e}")
except KreuzbergError as e:
    print(f"Extraction failed: {e}")
```
