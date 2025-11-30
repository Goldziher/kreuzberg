# Custom Hooks

Kreuzberg allows you to customize the extraction process through validation and post-processing hooks.

## Validation Hooks

Validation hooks allow you to validate extraction results and raise errors if they don't meet your criteria.

```python
from kreuzberg import extract_file, ExtractionConfig, ValidationError, ExtractionResult

# Define a validation hook
def validate_content_length(result: ExtractionResult) -> None:
    """Validate that the extracted content has a minimum length."""
    if len(result.content) < 10:
        raise ValidationError("Extracted content is too short (less than 10 characters)")

# Use the validation hook
async def extract_with_validation():
    config = ExtractionConfig(validators=[validate_content_length])

    result = await extract_file("document.pdf", config=config)
    return result
```

## Post-Processing Hooks

Post-processing hooks allow you to modify the extraction result before it's returned.

```python
from kreuzberg import extract_file, ExtractionConfig, ExtractionResult

# Define a post-processing hook
def clean_whitespace(result: ExtractionResult) -> ExtractionResult:
    """Clean up excessive whitespace in the extracted text."""
    import re

    # Replace multiple spaces with a single space
    cleaned_content = re.sub(r"\s+", " ", result.content)

    # Replace multiple newlines with a single newline
    cleaned_content = re.sub(r"\n+", "\n", cleaned_content)

    # Create a new result with the cleaned content
    return ExtractionResult(content=cleaned_content, mime_type=result.mime_type, metadata=result.metadata)

# Use the post-processing hook
async def extract_with_post_processing():
    config = ExtractionConfig(post_processing_hooks=[clean_whitespace])

    result = await extract_file("document.pdf", config=config)
    return result
```

## Combining Multiple Hooks

You can combine multiple validation and post-processing hooks:

```python
from kreuzberg import extract_file, ExtractionConfig, ExtractionResult, ValidationError

# Define validation hooks
def validate_content_length(result: ExtractionResult) -> None:
    if len(result.content) < 10:
        raise ValidationError("Extracted content is too short")

def validate_has_text(result: ExtractionResult) -> None:
    if not result.content.strip():
        raise ValidationError("Extracted content is empty or contains only whitespace")

# Define post-processing hooks
def clean_whitespace(result: ExtractionResult) -> ExtractionResult:
    import re

    cleaned_content = re.sub(r"\s+", " ", result.content)
    cleaned_content = re.sub(r"\n+", "\n", cleaned_content)

    return ExtractionResult(content=cleaned_content, mime_type=result.mime_type, metadata=result.metadata)

def normalize_text(result: ExtractionResult) -> ExtractionResult:
    """Normalize text by converting to lowercase and removing special characters."""
    import re

    # Convert to lowercase
    normalized = result.content.lower()

    # Remove special characters
    normalized = re.sub(r"[^\w\s]", "", normalized)

    return ExtractionResult(content=normalized, mime_type=result.mime_type, metadata=result.metadata)

# Use multiple hooks
async def extract_with_multiple_hooks():
    config = ExtractionConfig(
        validators=[validate_content_length, validate_has_text], post_processing_hooks=[clean_whitespace, normalize_text]
    )

    result = await extract_file("document.pdf", config=config)
    return result
```

## Advanced Example: Language Detection

Here's an example of using a post-processing hook to detect the language of extracted text:

```python
from kreuzberg import extract_file, ExtractionConfig, ExtractionResult

def detect_language(result: ExtractionResult) -> ExtractionResult:
    """Detect the language of the extracted text and add it to metadata."""
    try:
        # You need to install langdetect: pip install langdetect
        from langdetect import detect

        # Only detect if we have enough text
        if len(result.content) > 50:
            language = detect(result.content)

            # Create updated metadata with language information
            updated_metadata = dict(result.metadata)
            updated_metadata["detected_language"] = language

            return ExtractionResult(content=result.content, mime_type=result.mime_type, metadata=updated_metadata)
    except Exception:
        # If language detection fails, return the original result
        pass

    return result

async def extract_with_language_detection():
    config = ExtractionConfig(post_processing_hooks=[detect_language])

    result = await extract_file("document.pdf", config=config)

    if "detected_language" in result.metadata:
        print(f"Detected language: {result.metadata['detected_language']}")

    return result
```

## Best Practices

1. **Keep hooks simple**: Each hook should perform a single, well-defined task
1. **Handle exceptions**: Validation hooks should raise `ValidationError`, while post-processing hooks should handle their own exceptions
1. **Preserve metadata**: When modifying content in post-processing hooks, make sure to preserve the original metadata
1. **Order matters**: Post-processing hooks are applied in the order they are provided
1. **Be careful with mutations**: Create new `ExtractionResult` objects rather than modifying the original
1. **Document your hooks**: Include clear documentation for your custom hooks, especially if they're used across multiple projects
