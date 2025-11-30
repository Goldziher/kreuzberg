# Custom Extractors

The Kreuzberg document intelligence framework provides an extensible architecture through the `ExtractorRegistry` system. This plugin mechanism enables developers to extend document processing capabilities by implementing custom extractors for specialized formats or processing requirements.

## Creating a Custom Extractor

To create a custom extractor, you need to subclass the `Extractor` abstract base class and implement its required methods:

```python
from kreuzberg import ExtractorRegistry, ExtractionResult, ExtractionConfig
from kreuzberg._extractors._base import Extractor
from pathlib import Path

class CustomExtractor(Extractor):
    """Custom extractor for handling a specific file format."""

    # Define the MIME types this extractor supports
    SUPPORTED_MIME_TYPES = {"application/x-custom", "application/x-custom-format"}

    async def extract_bytes_async(self, content: bytes) -> ExtractionResult:
        """Asynchronously extract content from bytes."""
        # Implement your extraction logic here
        extracted_text = self._process_content(content)

        return ExtractionResult(content=extracted_text, mime_type=self.mime_type, metadata={"extractor": "CustomExtractor"})

    async def extract_path_async(self, path: Path) -> ExtractionResult:
        """Asynchronously extract content from a file path."""
        # Read the file and process it
        content = await self._read_file_async(path)
        return await self.extract_bytes_async(content)

    def extract_bytes_sync(self, content: bytes) -> ExtractionResult:
        """Synchronously extract content from bytes."""
        # Implement your extraction logic here
        extracted_text = self._process_content(content)

        return ExtractionResult(content=extracted_text, mime_type=self.mime_type, metadata={"extractor": "CustomExtractor"})

    def extract_path_sync(self, path: Path) -> ExtractionResult:
        """Synchronously extract content from a file path."""
        # Read the file and process it
        with open(path, "rb") as f:
            content = f.read()
        return self.extract_bytes_sync(content)

    def _process_content(self, content: bytes) -> str:
        """Process the content and extract text."""
        # Implement your content processing logic here
        # This is just an example
        return content.decode("utf-8", errors="ignore")

    async def _read_file_async(self, path: Path) -> bytes:
        """Read a file asynchronously."""
        # This is a simple implementation; you might want to use aiofiles in practice
        with open(path, "rb") as f:
            return f.read()
```

## Registering a Custom Extractor

Once you've created your custom extractor, you can register it with Kreuzberg using the `ExtractorRegistry`:

```python
from kreuzberg import ExtractorRegistry
from my_module import CustomExtractor

# Register the custom extractor
ExtractorRegistry.add_extractor(CustomExtractor)

# Now you can use it with standard extraction functions
from kreuzberg import extract_file

result = await extract_file("custom_document.xyz")
```

## Extractor Priority

Extractors are tried in the order they are registered. When extracting content, Kreuzberg will:

1. Try all user-registered extractors first (in the order they were added)
1. Then try all default extractors

This means your custom extractors take precedence over the built-in ones. If you want to override how a specific MIME type is handled, you can register a custom extractor that supports that MIME type.

## Removing an Extractor

You can remove a previously registered extractor:

```python
from kreuzberg import ExtractorRegistry
from my_module import CustomExtractor

# First register it
ExtractorRegistry.add_extractor(CustomExtractor)

# Later, remove it when no longer needed
ExtractorRegistry.remove_extractor(CustomExtractor)
```

## OCR Configuration in Custom Extractors

When creating custom extractors that need OCR capabilities, you can leverage Kreuzberg's OCR configuration options:

```python
from kreuzberg import ExtractorRegistry, ExtractionResult, ExtractionConfig, TesseractConfig, PSMMode
from kreuzberg._extractors._base import Extractor

class CustomImageExtractor(Extractor):
    """Custom extractor for image files with OCR capabilities."""

    SUPPORTED_MIME_TYPES = {"image/custom"}

    def extract_bytes_sync(self, content: bytes) -> ExtractionResult:
        # Get OCR configuration from the extraction config
        ocr_config = self.config.ocr_config

        if isinstance(ocr_config, TesseractConfig):
            # Access Tesseract-specific settings
            language = ocr_config.language  # Language model to use (e.g., "eng", "deu")
            psm = ocr_config.psm  # Page segmentation mode

            # Use these settings in your OCR processing
            # ...

        # Implement the rest of your extraction logic
        # ...

        return ExtractionResult(content="Extracted text", mime_type=self.mime_type, metadata={"ocr_engine": "tesseract"})
    # Implement other required methods...
```

## Example: Custom CSV Extractor

Here's a complete example of a custom CSV extractor that extracts text from CSV files:

```python
from kreuzberg import ExtractorRegistry, ExtractionResult, ExtractionConfig
from kreuzberg._extractors._base import Extractor
from pathlib import Path
import csv
import io

class CSVExtractor(Extractor):
    """Custom extractor for CSV files."""

    SUPPORTED_MIME_TYPES = {"text/csv", "application/csv"}

    async def extract_bytes_async(self, content: bytes) -> ExtractionResult:
        return self.extract_bytes_sync(content)

    async def extract_path_async(self, path: Path) -> ExtractionResult:
        with open(path, "rb") as f:
            content = f.read()
        return await self.extract_bytes_async(content)

    def extract_bytes_sync(self, content: bytes) -> ExtractionResult:
        text_content = content.decode("utf-8", errors="ignore")
        extracted_text = self._process_csv(text_content)

        return ExtractionResult(
            content=extracted_text, mime_type=self.mime_type, metadata={"extractor": "CSVExtractor", "format": "csv"}
        )

    def extract_path_sync(self, path: Path) -> ExtractionResult:
        with open(path, "rb") as f:
            content = f.read()
        return self.extract_bytes_sync(content)

    def _process_csv(self, csv_content: str) -> str:
        """Process CSV content and convert to plain text."""
        output = []
        csv_file = io.StringIO(csv_content)

        try:
            reader = csv.reader(csv_file)
            headers = next(reader, None)

            if headers:
                output.append(" | ".join(headers))
                output.append("-" * 40)

            for row in reader:
                output.append(" | ".join(row))

        except Exception as e:
            output.append(f"Error processing CSV: {str(e)}")

        return "\n".join(output)

# Register the custom extractor
ExtractorRegistry.add_extractor(CSVExtractor)
```

## Best Practices

1. **Define clear MIME type support**: Be specific about which MIME types your extractor supports
1. **Implement both sync and async methods**: Ensure your extractor works in both synchronous and asynchronous contexts
1. **Handle errors gracefully**: Catch and handle exceptions within your extractor methods
1. **Provide rich metadata**: Include useful information about the extraction process in the result metadata
1. **Test with various inputs**: Ensure your extractor works with a variety of file formats and edge cases
1. **Consider performance**: For large files, implement streaming or chunking to avoid memory issues
1. **Document your extractors**: Include clear documentation for your custom extractors
