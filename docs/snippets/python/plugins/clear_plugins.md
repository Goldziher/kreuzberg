```python title="Python"
from kreuzberg import (
    clear_document_extractors,
    clear_post_processors,
    clear_ocr_backends,
    clear_validators,
)

clear_post_processors()
clear_validators()
clear_ocr_backends()
clear_document_extractors()

print("All plugins cleared")
```
