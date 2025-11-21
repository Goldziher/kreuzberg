```python
from kreuzberg import (
    unregister_document_extractor,
    unregister_post_processor,
    unregister_ocr_backend,
    unregister_validator,
)

unregister_document_extractor("custom-json-extractor")
unregister_post_processor("word_count")
unregister_ocr_backend("cloud-ocr")
unregister_validator("min_length_validator")
```
