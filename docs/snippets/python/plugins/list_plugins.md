```python title="Python"
from kreuzberg import (
    list_document_extractors,
    list_post_processors,
    list_ocr_backends,
    list_validators,
)

extractors: list[str] = list_document_extractors()
processors: list[str] = list_post_processors()
ocr_backends: list[str] = list_ocr_backends()
validators: list[str] = list_validators()

print(f"Extractors: {extractors}")
print(f"Processors: {processors}")
print(f"OCR backends: {ocr_backends}")
print(f"Validators: {validators}")
```
