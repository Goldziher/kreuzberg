```python
from kreuzberg import (
    list_document_extractors,
    list_post_processors,
    list_ocr_backends,
    list_validators,
)

print("Extractors:", list_document_extractors())
print("Processors:", list_post_processors())
print("OCR backends:", list_ocr_backends())
print("Validators:", list_validators())
```
