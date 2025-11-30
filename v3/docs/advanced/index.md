# Advanced Topics

This section explores advanced capabilities of the Kreuzberg document intelligence framework, including extensibility mechanisms, performance optimization, and custom integration patterns.

## Topics

- [Performance Guide](performance.md) - Sync vs async performance analysis and optimization
- [Error Handling](error-handling.md) - Detailed strategies for handling extraction errors
- [Custom Hooks](custom-hooks.md) - Creating custom validation and post-processing hooks
- [Custom Extractors](custom-extractors.md) - Adding and removing custom extractors

## Advanced Configuration

Kreuzberg allows you to deeply customize the extraction process through:

1. **Validation Hooks**: Apply custom validation rules to extraction results
1. **Post-Processing Hooks**: Transform extraction results before they're returned
1. **Custom OCR Settings**: Fine-tune OCR behavior for specific document types

## Custom Validation Example

```python
from kreuzberg import extract_file, ExtractionConfig, ExtractionResult
from kreuzberg import ValidationError

# Define a custom validator that requires a minimum text length
def validate_text_length(result: ExtractionResult) -> None:
    if len(result.content) < 100:
        raise ValidationError("Extracted text is too short", context={"content_length": len(result.content)})

# Use the validator in extraction
async def extract_with_validation(file_path):
    config = ExtractionConfig(validators=[validate_text_length])

    try:
        result = await extract_file(file_path, config=config)
        return result.content
    except ValidationError as e:
        print(f"Validation failed: {e}")
        return None
```
