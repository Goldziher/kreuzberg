# Error Handling

Kreuzberg provides specific exception types to help you handle different error scenarios during text extraction.

## Exception Hierarchy

All Kreuzberg exceptions inherit from the base `KreuzbergError` class:

```text
KreuzbergError
├── MissingDependencyError
├── OCRError
├── ParsingError
└── ValidationError
```

## Handling Specific Exceptions

### Comprehensive Error Handling

```python
from kreuzberg import extract_file
from kreuzberg import KreuzbergError, MissingDependencyError, OCRError, ParsingError, ValidationError

async def safe_extract(path):
    try:
        result = await extract_file(path)
        return result.content
    except MissingDependencyError as e:
        # Handle missing system dependencies (Tesseract, Pandoc)
        print(f"Missing dependency: {e}")
        print("Please install the required dependencies.")
        # You might want to provide installation instructions here
    except OCRError as e:
        # Handle OCR processing failures
        print(f"OCR processing failed: {e}")
        # You might want to retry with different OCR settings
    except ParsingError as e:
        # Handle document parsing failures
        print(f"Document parsing failed: {e}")
        # You might want to try a different approach or format
    except ValidationError as e:
        # Handle validation errors in configuration
        print(f"Validation error: {e}")
        # Fix the configuration issue
    except KreuzbergError as e:
        # Catch-all for any other Kreuzberg-specific errors
        print(f"Extraction error: {e}")
    except Exception as e:
        # Handle unexpected errors
        print(f"Unexpected error: {e}")

    return None
```

### Simplified Error Handling

For simpler applications, you can catch just the base `KreuzbergError`:

```python
from kreuzberg import extract_file, KreuzbergError

async def simple_safe_extract(path):
    try:
        result = await extract_file(path)
        return result.content
    except KreuzbergError as e:
        print(f"Extraction failed: {e}")
        return None
```

## Common Error Scenarios

### Missing Dependencies

```python
try:
    result = await extract_file("document.pdf")
except MissingDependencyError as e:
    if "tesseract" in str(e).lower():
        print("Tesseract OCR is not installed. Please install it:")
        print("  - Ubuntu: sudo apt-get install tesseract-ocr")
        print("  - macOS: brew install tesseract")
        print("  - Windows: choco install tesseract")
    elif "pandoc" in str(e).lower():
        print("Pandoc is not installed. Please install it:")
        print("  - Ubuntu: sudo apt-get install pandoc")
        print("  - macOS: brew install pandoc")
        print("  - Windows: choco install pandoc")
```

### OCR Errors

```python
from kreuzberg import extract_file, OCRError, TesseractConfig, PSMMode

async def extract_with_fallback(path):
    # Try with default settings
    try:
        result = await extract_file(path)
        return result.content
    except OCRError:
        # Try with different OCR settings
        try:
            result = await extract_file(
                path, force_ocr=True, ocr_config=TesseractConfig(psm=PSMMode.SINGLE_BLOCK, language="eng")
            )
            return result.content
        except OCRError as e:
            print(f"OCR failed with all attempts: {e}")
            return None
```

### Validation Errors

```python
from kreuzberg import extract_file, ValidationError, TesseractConfig

async def extract_with_validation_handling():
    try:
        # This will raise a ValidationError - incompatible config
        result = await extract_file(
            "document.pdf", ocr_backend="easyocr", ocr_config=TesseractConfig(language="eng")  # Wrong config type for easyocr
        )
    except ValidationError as e:
        print(f"Configuration error: {e}")
        # Fix the configuration
        from kreuzberg import EasyOCRConfig

        result = await extract_file(
            "document.pdf", ocr_backend="easyocr", ocr_config=EasyOCRConfig(language="en")  # Correct config type
        )

    return result.content
```

## Best Practices

1. **Always use try/except**: Wrap extraction calls in try/except blocks to handle potential errors gracefully
1. **Provide helpful error messages**: Give users clear information about what went wrong and how to fix it
1. **Implement fallbacks**: When possible, try alternative approaches when the primary method fails
1. **Log detailed error information**: Include error details in logs for debugging
1. **Check dependencies upfront**: Verify that required dependencies are installed before attempting extraction

```python
import subprocess
from kreuzberg import extract_file

def check_dependencies():
    """Check if required dependencies are installed."""
    missing = []

    # Check for Tesseract
    try:
        subprocess.run(["tesseract", "--version"], capture_output=True, check=True)
    except (subprocess.SubprocessError, FileNotFoundError):
        missing.append("tesseract")

    # Check for Pandoc
    try:
        subprocess.run(["pandoc", "--version"], capture_output=True, check=True)
    except (subprocess.SubprocessError, FileNotFoundError):
        missing.append("pandoc")

    return missing

async def main():
    # Check dependencies before extraction
    missing_deps = check_dependencies()
    if missing_deps:
        print(f"Missing dependencies: {', '.join(missing_deps)}")
        print("Please install them before continuing.")
        return

    # Proceed with extraction
    try:
        result = await extract_file("document.pdf")
        print(result.content)
    except Exception as e:
        print(f"Extraction failed: {e}")
```
