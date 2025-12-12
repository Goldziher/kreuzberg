```python title="Python"
from kreuzberg import (
    unregister_document_extractor,
    unregister_post_processor,
    unregister_ocr_backend,
    unregister_validator,
)

names: list[str] = [
    "custom-json-extractor",
    "word_count",
    "cloud-ocr",
    "min_length_validator",
]

unregister_document_extractor(names[0])
unregister_post_processor(names[1])
unregister_ocr_backend(names[2])
unregister_validator(names[3])
```
