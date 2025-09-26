# Token Reduction

Kreuzberg provides a token reduction capability that helps optimize extracted text for processing by large language models or storage systems. This feature can significantly reduce the size of extracted content while preserving essential information and meaning.

## Overview

Token reduction processes extracted text to remove redundant content, normalize formatting, and optionally eliminate stopwords. This is particularly useful when working with token-limited APIs, implementing content summarization, or reducing storage costs for large document collections.

## Configuration

Token reduction is controlled through the `ExtractionConfig` class with the `token_reduction` parameter, which accepts a `TokenReductionConfig` object:

- `mode`: The reduction level - `"off"`, `"light"`, or `"moderate"` (default: `"off"`)
- `preserve_markdown`: Whether to preserve markdown structure during reduction (default: `True`)
- `language_hint`: Language hint for stopword removal in moderate mode (default: `None`)
- `custom_stopwords`: Additional stopwords per language (default: `None`)

⚠️ **Important Limitations:**

- Maximum text size: 10MB (10,000,000 characters)
- Language codes must match format: alphanumeric and hyphens only (e.g., "en", "en-US")

## Reduction Modes

### Off Mode

No reduction is applied - text is returned exactly as extracted.

### Light Mode

Applies formatting optimizations without changing semantic content:

- Removes HTML comments
- Normalizes excessive whitespace
- Compresses repeated punctuation
- Removes excessive newlines

**Performance**: ~10% character reduction, \<0.1ms processing time

### Moderate Mode

Includes all light mode optimizations plus stopword removal:

- Removes common stopwords in 64+ supported languages
- Preserves important words (short words, acronyms, words with numbers)
- Maintains markdown structure when enabled

**Performance**: ~35% character reduction, ~0.2ms processing time

## Basic Usage

```python
from kreuzberg import extract_file, ExtractionConfig, TokenReductionConfig

# Light mode - formatting optimization only
config = ExtractionConfig(token_reduction=TokenReductionConfig(mode="light"))
result = await extract_file("document.pdf", config=config)

# Moderate mode - formatting + stopword removal
config = ExtractionConfig(token_reduction=TokenReductionConfig(mode="moderate"))
result = await extract_file("document.pdf", config=config)

# The reduced content is available in result.content
print(f"Original length: {len(result.content)} characters")
```

## Language Support

Token reduction supports stopword removal in 64+ languages including:

- **Germanic**: English (en), German (de), Dutch (nl), Swedish (sv), Norwegian (no), Danish (da)
- **Romance**: Spanish (es), French (fr), Italian (it), Portuguese (pt), Romanian (ro), Catalan (ca)
- **Slavic**: Russian (ru), Polish (pl), Czech (cs), Bulgarian (bg), Croatian (hr), Slovak (sk)
- **Asian**: Chinese (zh), Japanese (ja), Korean (ko), Hindi (hi), Arabic (ar), Thai (th)
- **And many more**: Finnish, Hungarian, Greek, Hebrew, Turkish, Vietnamese, etc.

```python
from kreuzberg import extract_file, ExtractionConfig, TokenReductionConfig

# Specify language for better stopword detection
config = ExtractionConfig(token_reduction=TokenReductionConfig(mode="moderate", language_hint="es"))  # Spanish
result = await extract_file("documento.pdf", config=config)
```

## Custom Stopwords

You can add domain-specific stopwords for better reduction:

```python
from kreuzberg import extract_file, ExtractionConfig, TokenReductionConfig

config = ExtractionConfig(
    token_reduction=TokenReductionConfig(
        mode="moderate",
        custom_stopwords={"en": ["corporation", "company", "inc", "ltd"], "es": ["empresa", "sociedad", "limitada"]},
    )
)
result = await extract_file("business_document.pdf", config=config)
```

## Markdown Preservation

When `preserve_markdown=True` (default), the reducer maintains document structure:

```python
from kreuzberg import extract_file, ExtractionConfig, TokenReductionConfig

config = ExtractionConfig(
    token_reduction=TokenReductionConfig(mode="moderate", preserve_markdown=True)  # Preserves headers, lists, tables, code blocks
)
result = await extract_file("structured_document.md", config=config)
```

## Reduction Statistics

You can get detailed statistics about the reduction effectiveness:

```python
from kreuzberg._token_reduction import get_reduction_stats

# After extraction with token reduction
original_text = "The quick brown fox jumps over the lazy dog."
reduced_text = "quick brown fox jumps lazy dog."

stats = get_reduction_stats(original_text, reduced_text)
print(f"Character reduction: {stats['character_reduction_ratio']:.1%}")
print(f"Token reduction: {stats['token_reduction_ratio']:.1%}")
print(f"Original tokens: {stats['original_tokens']}")
print(f"Reduced tokens: {stats['reduced_tokens']}")
```

## Performance Benchmarks

Based on comprehensive testing across different text types:

### Light Mode Performance

- **Character Reduction**: 10.1% average (8.8% - 10.9% range)
- **Token Reduction**: 0% (preserves all words)
- **Processing Time**: 0.03ms average per document
- **Use Case**: Format cleanup without semantic changes

### Moderate Mode Performance

- **Character Reduction**: 35.3% average (11.4% - 62.3% range)
- **Token Reduction**: 33.7% average (1.9% - 57.6% range)
- **Processing Time**: 0.23ms average per document
- **Use Case**: Significant size reduction with preserved meaning

### Effectiveness by Content Type

- **Stopword-heavy text**: Up to 62% character reduction
- **Technical documentation**: 23-31% character reduction
- **Formal documents**: 29% character reduction
- **Scientific abstracts**: 30% character reduction
- **Minimal stopwords**: 11% character reduction (mostly formatting)

## Use Cases

### Large Language Model Integration

Reduce token costs and fit more content within model limits:

```python
from kreuzberg import extract_file, ExtractionConfig, TokenReductionConfig

# Optimize for LLM processing
config = ExtractionConfig(
    token_reduction=TokenReductionConfig(mode="moderate"), chunk_content=True, max_chars=3000  # Smaller chunks after reduction
)

result = await extract_file("large_report.pdf", config=config)

# Each chunk is now significantly smaller
for i, chunk in enumerate(result.chunks):
    # Process with LLM - now uses fewer tokens
    response = await llm_process(chunk)
```

### Content Storage Optimization

Reduce storage costs for large document collections:

```python
from kreuzberg import batch_extract_file, ExtractionConfig, TokenReductionConfig

# Process multiple documents with reduction
config = ExtractionConfig(token_reduction=TokenReductionConfig(mode="moderate"))

documents = ["doc1.pdf", "doc2.docx", "doc3.txt"]
results = await batch_extract_file(documents, config=config)

# Store reduced content - significant space savings
for doc, result in zip(documents, results):
    store_content(doc, result.content)  # 35% smaller on average
```

### Search Index Optimization

Create more efficient search indices:

```python
from kreuzberg import extract_file, ExtractionConfig, TokenReductionConfig

# Reduce content for search indexing
config = ExtractionConfig(
    token_reduction=TokenReductionConfig(mode="moderate", preserve_markdown=False)  # Remove structure for pure text search
)

result = await extract_file("document.pdf", config=config)

# Index the reduced content - smaller index, faster searches
search_index.add_document(doc_id, result.content)
```

## Best Practices

- **Choose the right mode**: Use `"light"` for format cleanup only, `"moderate"` for significant reduction
- **Preserve markdown for structured documents**: Keep `preserve_markdown=True` when document structure matters
- **Set language hints**: Specify `language_hint` for better stopword detection in non-English documents
- **Test with your content**: Effectiveness varies by document type - benchmark with your specific use case
- **Consider downstream processing**: Balance reduction benefits against potential information loss
- **Use custom stopwords judiciously**: Add domain-specific terms but avoid over-filtering

## Error Handling

```python
from kreuzberg import extract_file, ExtractionConfig, TokenReductionConfig
from kreuzberg.exceptions import ValidationError

try:
    config = ExtractionConfig(token_reduction=TokenReductionConfig(mode="moderate"))
    result = await extract_file("large_document.pdf", config=config)
except ValidationError as e:
    # Handle validation errors (e.g., text too large, invalid language code)
    print(f"Token reduction failed: {e}")
```

## Technical Details

The token reduction system uses:

- **Lazy loading**: Stopwords are loaded only when needed for specific languages
- **Pre-compiled regex patterns** for optimal performance
- **LRU caching** for frequently used languages (up to 16 cached)
- **Individual language files** for efficient memory usage
- **Intelligent markdown parsing** to preserve document structure
- **Security validation** including text size limits and language code validation
- **Efficient stopword management** with support for custom additions

The reduction process is highly optimized and adds minimal overhead to the extraction pipeline.
