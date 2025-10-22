# Python PostProcessor Development

PostProcessors transform extraction results after initial extraction. This guide covers implementing custom post-processors in Python.

## Overview

PostProcessors are the most flexible plugin type in Kreuzberg. They can:
- Modify extracted content
- Add or enrich metadata
- Filter or clean results
- Perform async operations (API calls, database queries)

## Basic PostProcessor

### Minimal Implementation

```python
from kreuzberg import register_post_processor, ExtractionResult

class SimplePostProcessor:
    def process(self, result: ExtractionResult) -> ExtractionResult:
        """Process the extraction result."""
        # Modify the result
        result.metadata["processed"] = True
        return result

    def name(self) -> str:
        """Return the processor name."""
        return "simple_processor"

# Register the processor
register_post_processor(SimplePostProcessor())
```

### Key Requirements

1. **`process()` method**: Takes `ExtractionResult`, returns modified `ExtractionResult`
2. **`name()` method**: Returns unique string identifier
3. **Thread-safe**: Must handle concurrent calls
4. **Registration**: Call `register_post_processor()` to activate

## Complete Example: Metadata Enricher

```python
from kreuzberg import register_post_processor, ExtractionResult
from datetime import datetime
from typing import Dict, Any

class MetadataEnricher:
    """Adds comprehensive metadata to extraction results."""

    def __init__(self, add_timestamp: bool = True, add_stats: bool = True):
        self.add_timestamp = add_timestamp
        self.add_stats = add_stats

    def process(self, result: ExtractionResult) -> ExtractionResult:
        """Enrich result with metadata."""
        if self.add_timestamp:
            result.metadata["processed_at"] = datetime.now().isoformat()

        if self.add_stats:
            # Add content statistics
            result.metadata.update(self._calculate_stats(result))

        return result

    def _calculate_stats(self, result: ExtractionResult) -> Dict[str, Any]:
        """Calculate content statistics."""
        content = result.content
        return {
            "char_count": len(content),
            "word_count": len(content.split()),
            "line_count": len(content.splitlines()),
            "table_count": len(result.tables),
            "has_content": len(content.strip()) > 0,
        }

    def name(self) -> str:
        return "metadata_enricher"

# Register with configuration
register_post_processor(MetadataEnricher(
    add_timestamp=True,
    add_stats=True
))
```

### Usage

```python
from kreuzberg import extract_file_sync, ExtractionConfig

# Extract with our post-processor
result = extract_file_sync("document.pdf", config=ExtractionConfig())

# Check added metadata
print(result.metadata["processed_at"])
print(result.metadata["word_count"])
```

## Async PostProcessor

For I/O-bound operations (API calls, database queries), use async:

```python
from kreuzberg import register_post_processor, ExtractionResult
import httpx
import asyncio

class AsyncAPIEnricher:
    """Enrich results with external API data."""

    def __init__(self, api_key: str, api_url: str):
        self.api_key = api_key
        self.api_url = api_url
        self._client = None

    async def process(self, result: ExtractionResult) -> ExtractionResult:
        """Process with async API call."""
        # Initialize client on first use
        if self._client is None:
            self._client = httpx.AsyncClient()

        try:
            # Call external API
            response = await self._client.post(
                self.api_url,
                json={
                    "text": result.content[:1000],  # Send preview
                    "metadata": result.metadata,
                },
                headers={"Authorization": f"Bearer {self.api_key}"},
                timeout=5.0,
            )
            response.raise_for_status()

            # Add API response to metadata
            api_data = response.json()
            result.metadata["api_enrichment"] = api_data

        except Exception as e:
            # Handle errors gracefully
            result.metadata["api_enrichment_error"] = str(e)

        return result

    def name(self) -> str:
        return "async_api_enricher"

    async def cleanup(self):
        """Clean up resources."""
        if self._client:
            await self._client.aclose()

# Register
enricher = AsyncAPIEnricher(
    api_key="your_key",
    api_url="https://api.example.com/enrich"
)
register_post_processor(enricher)
```

### Using with Async Extraction

```python
from kreuzberg import extract_file, ExtractionConfig

async def main():
    result = await extract_file("document.pdf", config=ExtractionConfig())
    print(result.metadata.get("api_enrichment"))

    # Cleanup
    await enricher.cleanup()

import asyncio
asyncio.run(main())
```

## Content Transformation Examples

### PII Redaction

```python
from kreuzberg import register_post_processor, ExtractionResult
import re

class PIIRedactor:
    """Redact personally identifiable information."""

    def __init__(self, redact_emails: bool = True, redact_phones: bool = True,
                 redact_ssn: bool = False):
        self.redact_emails = redact_emails
        self.redact_phones = redact_phones
        self.redact_ssn = redact_ssn

    def process(self, result: ExtractionResult) -> ExtractionResult:
        """Redact PII from content."""
        content = result.content

        if self.redact_emails:
            content = self._redact_emails(content)

        if self.redact_phones:
            content = self._redact_phones(content)

        if self.redact_ssn:
            content = self._redact_ssn(content)

        result.content = content
        result.metadata["pii_redacted"] = True
        return result

    def _redact_emails(self, text: str) -> str:
        """Redact email addresses."""
        return re.sub(
            r'\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b',
            '[EMAIL REDACTED]',
            text
        )

    def _redact_phones(self, text: str) -> str:
        """Redact phone numbers (US format)."""
        # (555) 123-4567, 555-123-4567, 555.123.4567
        patterns = [
            r'\(\d{3}\)\s*\d{3}[-.\s]?\d{4}',
            r'\d{3}[-.\s]\d{3}[-.\s]\d{4}',
        ]
        for pattern in patterns:
            text = re.sub(pattern, '[PHONE REDACTED]', text)
        return text

    def _redact_ssn(self, text: str) -> str:
        """Redact Social Security Numbers."""
        return re.sub(
            r'\b\d{3}-\d{2}-\d{4}\b',
            '[SSN REDACTED]',
            text
        )

    def name(self) -> str:
        return "pii_redactor"

register_post_processor(PIIRedactor(
    redact_emails=True,
    redact_phones=True,
    redact_ssn=True
))
```

### Text Normalization

```python
from kreuzberg import register_post_processor, ExtractionResult
import unicodedata

class TextNormalizer:
    """Normalize and clean extracted text."""

    def __init__(self, normalize_whitespace: bool = True,
                 normalize_unicode: bool = True,
                 remove_empty_lines: bool = True):
        self.normalize_whitespace = normalize_whitespace
        self.normalize_unicode = normalize_unicode
        self.remove_empty_lines = remove_empty_lines

    def process(self, result: ExtractionResult) -> ExtractionResult:
        """Normalize text content."""
        content = result.content

        if self.normalize_unicode:
            # Normalize Unicode to NFC form
            content = unicodedata.normalize('NFC', content)

        if self.normalize_whitespace:
            # Replace multiple spaces with single space
            content = re.sub(r' +', ' ', content)
            # Replace multiple newlines with max 2
            content = re.sub(r'\n{3,}', '\n\n', content)

        if self.remove_empty_lines:
            # Remove lines with only whitespace
            lines = [line for line in content.splitlines() if line.strip()]
            content = '\n'.join(lines)

        result.content = content
        result.metadata["text_normalized"] = True
        return result

    def name(self) -> str:
        return "text_normalizer"

register_post_processor(TextNormalizer())
```

### Content Summarization

```python
from kreuzberg import register_post_processor, ExtractionResult

class ContentSummarizer:
    """Add content summary to metadata."""

    def __init__(self, max_summary_length: int = 500):
        self.max_summary_length = max_summary_length

    def process(self, result: ExtractionResult) -> ExtractionResult:
        """Generate content summary."""
        content = result.content

        # Extract first N characters as summary
        summary = content[:self.max_summary_length]

        # Try to break at sentence boundary
        if len(content) > self.max_summary_length:
            # Find last period or newline
            last_period = summary.rfind('.')
            last_newline = summary.rfind('\n')
            break_point = max(last_period, last_newline)

            if break_point > 0:
                summary = summary[:break_point + 1]
            else:
                summary += "..."

        result.metadata["summary"] = summary.strip()
        result.metadata["is_truncated"] = len(content) > self.max_summary_length

        return result

    def name(self) -> str:
        return "content_summarizer"

register_post_processor(ContentSummarizer(max_summary_length=500))
```

## Advanced Patterns

### Conditional Processing

Process differently based on result properties:

```python
from kreuzberg import register_post_processor, ExtractionResult

class ConditionalProcessor:
    """Process based on content characteristics."""

    def process(self, result: ExtractionResult) -> ExtractionResult:
        """Apply conditional processing."""
        # Check MIME type
        mime_type = result.metadata.get("mime_type", "")

        if mime_type == "application/pdf":
            self._process_pdf(result)
        elif mime_type.startswith("image/"):
            self._process_image(result)

        # Check content length
        if len(result.content) < 100:
            result.metadata["content_quality"] = "low"
        elif len(result.content) < 1000:
            result.metadata["content_quality"] = "medium"
        else:
            result.metadata["content_quality"] = "high"

        return result

    def _process_pdf(self, result: ExtractionResult):
        """PDF-specific processing."""
        result.metadata["format"] = "pdf"
        # Add PDF-specific metadata

    def _process_image(self, result: ExtractionResult):
        """Image-specific processing."""
        result.metadata["format"] = "image"
        # Add image-specific metadata

    def name(self) -> str:
        return "conditional_processor"

register_post_processor(ConditionalProcessor())
```

### Chaining with State

Maintain state across multiple processors:

```python
from kreuzberg import register_post_processor, ExtractionResult
from typing import Dict, Any

class StatefulProcessor:
    """Processor that maintains processing state."""

    def __init__(self):
        self.processing_count = 0
        self.total_words = 0

    def process(self, result: ExtractionResult) -> ExtractionResult:
        """Process with state tracking."""
        # Update state
        self.processing_count += 1
        words = len(result.content.split())
        self.total_words += words

        # Add state to metadata
        result.metadata["processing_sequence"] = self.processing_count
        result.metadata["cumulative_words"] = self.total_words

        return result

    def get_stats(self) -> Dict[str, Any]:
        """Get processing statistics."""
        return {
            "total_processed": self.processing_count,
            "total_words": self.total_words,
            "avg_words": self.total_words / max(1, self.processing_count),
        }

    def name(self) -> str:
        return "stateful_processor"

# Create and register
processor = StatefulProcessor()
register_post_processor(processor)

# Later, get stats
stats = processor.get_stats()
print(f"Processed {stats['total_processed']} documents")
```

### Error Recovery

Handle errors gracefully:

```python
from kreuzberg import register_post_processor, ExtractionResult
import logging

logger = logging.getLogger(__name__)

class RobustProcessor:
    """Processor with comprehensive error handling."""

    def process(self, result: ExtractionResult) -> ExtractionResult:
        """Process with error recovery."""
        try:
            # Risky operation
            self._risky_operation(result)
        except ValueError as e:
            # Handle specific error
            logger.warning(f"ValueError in processing: {e}")
            result.metadata["processing_warning"] = str(e)
        except Exception as e:
            # Handle unexpected errors
            logger.error(f"Unexpected error in processing: {e}")
            result.metadata["processing_error"] = str(e)
            # Continue processing, don't fail

        return result

    def _risky_operation(self, result: ExtractionResult):
        """Operation that might fail."""
        # Implementation that could raise exceptions
        pass

    def name(self) -> str:
        return "robust_processor"

register_post_processor(RobustProcessor())
```

## Testing PostProcessors

### Unit Testing

```python
import pytest
from kreuzberg import ExtractionResult
from my_plugin import MetadataEnricher

def test_metadata_enricher_adds_stats():
    """Test that enricher adds statistics."""
    # Create test result
    result = ExtractionResult(
        content="Hello world test",
        metadata={},
        tables=[],
    )

    # Process
    processor = MetadataEnricher(add_stats=True)
    processed = processor.process(result)

    # Assert
    assert "word_count" in processed.metadata
    assert processed.metadata["word_count"] == 3
    assert processed.metadata["char_count"] == 16

def test_metadata_enricher_adds_timestamp():
    """Test that enricher adds timestamp."""
    result = ExtractionResult(content="test", metadata={}, tables=[])

    processor = MetadataEnricher(add_timestamp=True)
    processed = processor.process(result)

    assert "processed_at" in processed.metadata
    assert isinstance(processed.metadata["processed_at"], str)

@pytest.mark.asyncio
async def test_async_processor():
    """Test async processor."""
    from my_plugin import AsyncAPIEnricher

    result = ExtractionResult(content="test", metadata={}, tables=[])

    processor = AsyncAPIEnricher(api_key="test", api_url="http://example.com")
    processed = await processor.process(result)

    # Assert results
    assert processed is not None
```

### Integration Testing

```python
from kreuzberg import extract_file_sync, ExtractionConfig, register_post_processor
from my_plugin import MetadataEnricher

def test_integration_with_extraction():
    """Test processor in full extraction pipeline."""
    # Register processor
    register_post_processor(MetadataEnricher())

    # Extract file
    result = extract_file_sync("test_document.pdf", config=ExtractionConfig())

    # Verify processor ran
    assert "processed_at" in result.metadata
    assert "word_count" in result.metadata
```

## Best Practices

### Performance

1. **Avoid expensive operations**: Cache results when possible
2. **Use async for I/O**: Don't block on network/disk operations
3. **Minimize copying**: Modify results in-place when safe
4. **Profile your code**: Measure impact on extraction time

### Thread Safety

PostProcessors may be called concurrently:

```python
from threading import Lock

class ThreadSafeProcessor:
    """Thread-safe processor with shared state."""

    def __init__(self):
        self._lock = Lock()
        self._counter = 0

    def process(self, result: ExtractionResult) -> ExtractionResult:
        """Thread-safe processing."""
        with self._lock:
            self._counter += 1
            result.metadata["sequence"] = self._counter

        return result

    def name(self) -> str:
        return "thread_safe_processor"
```

### Error Handling

1. **Don't fail silently**: Log errors and warnings
2. **Add error metadata**: Help users debug issues
3. **Let system errors bubble**: Don't catch OSError/RuntimeError
4. **Graceful degradation**: Continue processing when possible

### Documentation

Document your processor:

```python
class WellDocumentedProcessor:
    """One-line summary of what this processor does.

    Detailed description of the processor's behavior,
    including any side effects or requirements.

    Args:
        param1: Description of param1
        param2: Description of param2

    Example:
        >>> from kreuzberg import register_post_processor
        >>> processor = WellDocumentedProcessor(param1=True)
        >>> register_post_processor(processor)
    """

    def __init__(self, param1: bool = True, param2: str = "default"):
        self.param1 = param1
        self.param2 = param2

    def process(self, result: ExtractionResult) -> ExtractionResult:
        """Process the extraction result.

        Args:
            result: The extraction result to process

        Returns:
            Modified extraction result

        Note:
            This processor modifies the content in-place.
        """
        # Implementation
        return result

    def name(self) -> str:
        return "well_documented_processor"
```

## Common Pitfalls

### 1. Modifying Immutable Fields

```python
# ❌ Wrong - ExtractionResult fields may be immutable
result.content = new_content  # May fail

# ✅ Correct - Create new result if needed
from dataclasses import replace
result = replace(result, content=new_content)
```

### 2. Forgetting to Return Result

```python
# ❌ Wrong - missing return
def process(self, result: ExtractionResult) -> ExtractionResult:
    result.metadata["processed"] = True
    # Missing return!

# ✅ Correct - always return
def process(self, result: ExtractionResult) -> ExtractionResult:
    result.metadata["processed"] = True
    return result
```

### 3. Blocking Async Operations

```python
# ❌ Wrong - blocking in async processor
async def process(self, result: ExtractionResult) -> ExtractionResult:
    response = requests.get("http://api.example.com")  # Blocking!
    return result

# ✅ Correct - use async HTTP client
async def process(self, result: ExtractionResult) -> ExtractionResult:
    async with httpx.AsyncClient() as client:
        response = await client.get("http://api.example.com")
    return result
```

## Next Steps

- [Python OCR Backend Development](python-ocr-backend.md) - Implement custom OCR
- [Plugin Development Overview](overview.md) - Compare plugin types
- [API Reference](../api-reference/python/) - Complete API documentation
