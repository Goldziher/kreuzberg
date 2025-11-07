# Custom PostProcessors in Kreuzberg

Kreuzberg allows you to register custom post-processors written in Python that will be executed as part of the extraction pipeline.

## Quick Start

```python
from kreuzberg import PostProcessorProtocol, register_post_processor, ExtractionResult

class MyProcessor:
    def name(self) -> str:
        return "my_processor"

    def process(self, result: ExtractionResult) -> ExtractionResult:
        # Add custom metadata
        result.metadata["custom_field"] = "custom_value"
        return result

    def processing_stage(self) -> str:
        return "middle"  # or "early" or "late"

# Register the processor
register_post_processor(MyProcessor())

# Now extract - your processor runs automatically
from kreuzberg import extract_file_sync

result = extract_file_sync("document.pdf")
print(result.metadata.get("custom_field"))  # "custom_value"
```

## PostProcessor Protocol

All custom processors must implement these methods:

### Required Methods

#### `name() -> str`

Returns a unique identifier for your processor.

```python
def name(self) -> str:
    return "my_custom_processor"
```

#### `process(result: ExtractionResult) -> ExtractionResult`

The main processing logic. Receives an ExtractionResult and returns the modified result.

```python
def process(self, result: ExtractionResult) -> ExtractionResult:
    # Access extracted content
    text = result.content

    # Add custom metadata
    result.metadata["word_count"] = len(text.split())

    # Return modified result
    return result
```

### Optional Methods

#### `processing_stage() -> Literal["early", "middle", "late"]`

Controls when your processor runs in the pipeline. Defaults to "middle".

- **early**: Runs first (e.g., language detection)
- **middle**: Runs in the middle (default, most processors)
- **late**: Runs last (e.g., final formatting)

```python
def processing_stage(self) -> str:
    return "late"
```

#### `initialize() -> None`

Called once when the processor is registered. Use for expensive initialization (loading ML models, etc.).

```python
def initialize(self) -> None:
    self.model = load_ml_model()
```

#### `shutdown() -> None`

Called when the processor is unregistered. Use for cleanup.

```python
def shutdown(self) -> None:
    self.model = None
```

## Real-World Examples

### Example 1: Word Statistics Processor

```python
class WordStatsProcessor:
    """Adds word count and sentence count statistics."""

    def name(self) -> str:
        return "word_stats"

    def process(self, result: ExtractionResult) -> ExtractionResult:
        words = result.content.split()
        sentences = result.content.count(".") + result.content.count("!") + result.content.count("?")

        result.metadata["word_count"] = len(words)
        result.metadata["sentence_count"] = sentences
        result.metadata["avg_word_length"] = sum(len(w) for w in words) / len(words) if words else 0

        return result

    def processing_stage(self) -> str:
        return "middle"
```

### Example 2: Custom Sentiment Analysis

```python
class SentimentProcessor:
    """Adds sentiment analysis to extraction results."""

    def name(self) -> str:
        return "sentiment_analysis"

    def initialize(self) -> None:
        # Load sentiment model only once
        from transformers import pipeline

        self.analyzer = pipeline("sentiment-analysis")

    def process(self, result: ExtractionResult) -> ExtractionResult:
        # Analyze sentiment (truncate long texts)
        text = result.content[:512]
        sentiment = self.analyzer(text)[0]

        result.metadata["sentiment"] = {"label": sentiment["label"], "score": float(sentiment["score"])}

        return result

    def processing_stage(self) -> str:
        return "late"  # Run after other processors

    def shutdown(self) -> None:
        self.analyzer = None
```

### Example 3: Language-Specific Processing

```python
class LanguageSpecificProcessor:
    """Applies different processing based on detected language."""

    def name(self) -> str:
        return "language_specific"

    def process(self, result: ExtractionResult) -> ExtractionResult:
        # Use language detection from metadata (if available)
        lang = result.metadata.get("detected_language", "unknown")

        if lang == "en":
            # English-specific processing
            result.metadata["language_note"] = "English document detected"
        elif lang == "de":
            # German-specific processing
            result.metadata["language_note"] = "Deutsches Dokument erkannt"
        else:
            result.metadata["language_note"] = f"Document in {lang}"

        return result

    def processing_stage(self) -> str:
        return "middle"  # After language detection
```

## Best Practices

### 1. Keep Processing Fast

Processors run on every extraction. Avoid slow operations when possible.

```python
# ❌ Bad: Slow on every call
def process(self, result):
    model = load_expensive_model()  # Loads every time!
    return process_with_model(result, model)

# ✅ Good: Load once in initialize()
def initialize(self):
    self.model = load_expensive_model()  # Load once

def process(self, result):
    return process_with_model(result, self.model)
```

### 2. Don't Modify Existing Metadata

Add new keys, don't overwrite existing ones.

```python
# ❌ Bad: Might overwrite important metadata
def process(self, result):
    result.metadata = {"my_field": "value"}  # Wipes existing metadata!
    return result

# ✅ Good: Add to existing metadata
def process(self, result):
    result.metadata["my_field"] = "value"  # Adds new field
    return result
```

### 3. Handle Errors Gracefully

Don't crash the extraction pipeline on errors.

```python
def process(self, result):
    try:
        # Your processing logic
        result.metadata["custom"] = analyze(result.content)
    except Exception as e:
        # Log error but don't crash
        result.metadata["custom_error"] = str(e)

    return result
```

### 4. Thread Safety

If your processor uses shared state, ensure it's thread-safe.

```python
import threading

class ThreadSafeProcessor:
    def __init__(self):
        self.lock = threading.Lock()
        self.counter = 0

    def process(self, result):
        with self.lock:
            self.counter += 1
            result.metadata["process_count"] = self.counter
        return result
```

## Integration with PostProcessorConfig

Use `PostProcessorConfig` to enable/disable processors:

```python
from kreuzberg import ExtractionConfig, PostProcessorConfig

# Enable only specific processors
config = ExtractionConfig(
    postprocessor=PostProcessorConfig(
        enabled=True,
        enabled_processors=["my_processor", "word_stats"],  # Whitelist
    )
)

# Or disable specific processors
config = ExtractionConfig(
    postprocessor=PostProcessorConfig(
        enabled=True,
        disabled_processors=["slow_processor"],  # Blacklist
    )
)

result = extract_file_sync("document.pdf", config=config)
```

## See Also

- Full example: `examples/custom_postprocessor.py`
- Protocol definition: `kreuzberg/postprocessors/protocol.py`
- OCR backend registration: `kreuzberg/ocr/`
