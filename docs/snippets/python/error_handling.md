```python
from kreuzberg import (
    extract_file_sync,
    ExtractionConfig,
    ValidationError,
    ParsingError,
    OCRError,
    MissingDependencyError,
    KreuzbergError,
)

try:
    result = extract_file_sync('document.pdf', config=ExtractionConfig())
    print(result.content)
except ValidationError as exc:
    print(f"Invalid configuration: {exc}")
except ParsingError as exc:
    print(f"Failed to parse document: {exc}")
except OCRError as exc:
    print(f"OCR processing failed: {exc}")
except MissingDependencyError as exc:
    print(f"Missing dependency: {exc}")
except KreuzbergError as exc:
    print(f"Extraction error: {exc}")
except OSError as exc:
    print(f"System error: {exc}")
```
