```python
from kreuzberg import (
    ExtractionResult,
    ValidationError,
    extract_file_sync,
    register_validator,
    unregister_validator,
)


def min_length_validator(result: ExtractionResult) -> None:
    if len(result.content) < 50:
        raise ValidationError(f"Content too short: {len(result.content)}")


register_validator("min_length", min_length_validator)

result = extract_file_sync("document.pdf")
print(f"Validated content length: {len(result.content)}")

unregister_validator("min_length")
```
