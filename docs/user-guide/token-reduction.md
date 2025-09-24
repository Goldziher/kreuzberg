# Token Reduction

Kreuzberg provides a high-performance token reduction capability powered by modern Rust implementation that helps optimize extracted text for processing by large language models or storage systems. This feature can significantly reduce the size of extracted content while preserving essential information and meaning.

## Overview

Token reduction processes extracted text to remove redundant content, normalize formatting, and optionally eliminate stopwords using advanced SIMD-optimized algorithms. This is particularly useful when working with token-limited APIs, implementing content summarization, or reducing storage costs for large document collections.

The system features:

- **High Performance**: Rust-based implementation with SIMD optimization for fast processing
- **Language Agnostic**: Universal patterns work across 64+ languages without hardcoded assumptions
- **Advanced Markdown Awareness**: Sophisticated preservation of headers, lists, tables, and code blocks
- **Complex Pattern Handling**: Intelligent normalization of punctuation including mixed sequences
- **Enhanced Security**: Comprehensive input validation and DoS protection

## Configuration

Token reduction is controlled through the `ExtractionConfig` class with the `token_reduction` parameter, which accepts a `TokenReductionConfig` object:

- `mode`: The reduction level - `"off"`, `"light"`, `"moderate"`, or `"aggressive"` (default: `"off"`)
- `preserve_markdown`: Whether to preserve markdown structure during reduction (default: `True`)
- `language_hint`: Language hint for stopword removal in moderate/aggressive modes (default: `None`)
- `custom_stopwords`: Additional stopwords per language (default: `None`)

⚠️ **Important Limitations:**

- Maximum text size: 2MB (2,097,152 characters) for DoS protection
- Language codes must match format: alphanumeric, hyphens, and underscores only (e.g., "en", "en-US", "zh_CN")
- Security validation prevents potentially dangerous inputs

## Reduction Modes

### Off Mode

No reduction is applied - text is returned exactly as extracted.

### Light Mode

Applies formatting optimizations without changing semantic content:

- Removes HTML comments with proper whitespace cleanup
- Normalizes excessive whitespace and newlines (3+ newlines → 2 newlines)
- Compresses repeated punctuation (including mixed patterns like `?!?!?!`)
- Preserves code blocks and markdown structure when enabled
- SIMD-optimized punctuation processing

**Performance**: ~10% character reduction, \<0.1ms processing time

### Moderate Mode

Includes all light mode optimizations plus intelligent stopword removal:

- Removes common stopwords in 64+ supported languages using language-agnostic patterns
- Preserves important words (short words, acronyms, words with numbers)
- Advanced markdown structure preservation (headers, lists, tables remain untouched)
- Line-by-line processing to protect markdown elements from stopword removal
- Maintains document hierarchy and formatting

**Performance**: ~35% character reduction, ~0.2ms processing time

### Aggressive Mode

Maximum reduction while preserving document meaning and structure:

- All moderate mode features plus enhanced semantic filtering
- Advanced pattern recognition for technical terms and proper nouns
- Universal tokenization supporting CJK languages without whitespace
- Statistical frequency analysis for content optimization
- Parallel processing for improved performance on large texts

**Performance**: ~50-60% character reduction, ~0.3ms processing time

## Basic Usage

```python
from kreuzberg import extract_file, ExtractionConfig, TokenReductionConfig

# Light mode - formatting optimization only
config = ExtractionConfig(token_reduction=TokenReductionConfig(mode="light"))
result = await extract_file("document.pdf", config=config)

# Moderate mode - formatting + stopword removal
config = ExtractionConfig(token_reduction=TokenReductionConfig(mode="moderate"))
result = await extract_file("document.pdf", config=config)

# Aggressive mode - maximum reduction with semantic awareness
config = ExtractionConfig(token_reduction=TokenReductionConfig(mode="aggressive"))
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

You can get detailed statistics about the reduction effectiveness using the Rust-powered statistics engine:

```python
from kreuzberg._token_reduction import get_reduction_statistics

# After extraction with token reduction
original_text = "The quick brown fox jumps over the lazy dog."
reduced_text = "quick brown fox jumps lazy dog."

stats = get_reduction_statistics(original_text, reduced_text)
print(f"Character reduction: {stats['character_reduction_ratio']:.1%}")
print(f"Token reduction: {stats['token_reduction_ratio']:.1%}")
print(f"Original characters: {stats['original_characters']}")
print(f"Reduced characters: {stats['reduced_characters']}")
print(f"Original tokens: {stats['original_tokens']}")
print(f"Reduced tokens: {stats['reduced_tokens']}")
```

## Performance Benchmarks

Based on comprehensive testing with the new Rust implementation across different text types:

### Light Mode Performance

- **Character Reduction**: 10.1% average (8.8% - 10.9% range)
- **Token Reduction**: 0% (preserves all words)
- **Processing Time**: \<0.1ms average per document (SIMD-optimized)
- **Use Case**: Format cleanup without semantic changes

### Moderate Mode Performance

- **Character Reduction**: 35.3% average (11.4% - 62.3% range)
- **Token Reduction**: 33.7% average (1.9% - 57.6% range)
- **Processing Time**: ~0.2ms average per document
- **Use Case**: Significant size reduction with preserved meaning

### Aggressive Mode Performance

- **Character Reduction**: 50-60% average (45% - 70% range)
- **Token Reduction**: 45-55% average (40% - 65% range)
- **Processing Time**: ~0.3ms average per document (parallel processing)
- **Use Case**: Maximum reduction with semantic awareness

### Effectiveness by Content Type

- **Stopword-heavy text**: Up to 70% character reduction (aggressive mode)
- **Technical documentation**: 25-45% character reduction
- **Formal documents**: 35-50% character reduction
- **Scientific abstracts**: 40-55% character reduction
- **Minimal stopwords**: 12-20% character reduction (mostly formatting)

### Rust Performance Improvements

- **SIMD Optimization**: 3-5x faster punctuation processing
- **Parallel Processing**: 2-4x speedup on multi-core systems for large texts
- **Memory Efficiency**: 40-60% lower memory usage compared to Python implementation
- **Cache Optimization**: Intelligent LRU caching for stopword dictionaries

## Use Cases

### Large Language Model Integration

Reduce token costs and fit more content within model limits:

```python
from kreuzberg import extract_file, ExtractionConfig, TokenReductionConfig

# Optimize for LLM processing with aggressive reduction for maximum token savings
config = ExtractionConfig(
    token_reduction=TokenReductionConfig(mode="aggressive"),
    chunk_content=True,
    max_chars=2000,  # Even smaller chunks after aggressive reduction
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

# Process multiple documents with aggressive reduction for maximum storage savings
config = ExtractionConfig(token_reduction=TokenReductionConfig(mode="aggressive"))

documents = ["doc1.pdf", "doc2.docx", "doc3.txt"]
results = await batch_extract_file(documents, config=config)

# Store reduced content - significant space savings
for doc, result in zip(documents, results):
    store_content(doc, result.content)  # 50-60% smaller on average with aggressive mode
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

- **Choose the right mode**: Use `"light"` for format cleanup only, `"moderate"` for balanced reduction, `"aggressive"` for maximum compression
- **Preserve markdown for structured documents**: Keep `preserve_markdown=True` when document structure matters
- **Set language hints**: Specify `language_hint` for better stopword detection in non-English documents
- **Test with your content**: Effectiveness varies by document type - benchmark with your specific use case
- **Consider downstream processing**: Balance reduction benefits against potential information loss
- **Use custom stopwords judiciously**: Add domain-specific terms but avoid over-filtering
- **Leverage aggressive mode**: For maximum token savings in LLM applications where slight semantic loss is acceptable
- **Monitor performance**: The Rust implementation provides sub-millisecond processing even for large documents

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

The Rust-powered token reduction system uses advanced optimization techniques:

### Core Architecture

- **Rust Implementation**: High-performance core with Python bindings via PyO3
- **SIMD Optimization**: Vectorized text processing using memchr and custom SIMD algorithms
- **Parallel Processing**: Multi-threaded text chunking and processing for large documents
- **Memory Safety**: Rust's ownership system prevents memory leaks and buffer overflows

### Performance Optimizations

- **Lazy loading**: Stopwords are loaded only when needed for specific languages
- **Pre-compiled regex patterns** with Rust's regex crate for optimal performance
- **LRU caching** for frequently used languages (up to 16 cached)
- **Individual language files** for efficient memory usage
- **Zero-copy operations**: Minimize memory allocations during text processing

### Advanced Features

- **Intelligent markdown parsing** with line-by-line processing to preserve document structure
- **Complex punctuation handling**: Advanced pattern recognition for mixed punctuation sequences
- **Universal tokenization**: CJK language support without whitespace dependency
- **Semantic clustering**: Advanced token importance scoring in aggressive mode
- **Statistical frequency analysis**: TF-based scoring for content optimization

### Security & Validation

- **Input validation**: Comprehensive text size limits (2MB) and language code validation
- **DoS protection**: Built-in safeguards against malicious input
- **Unicode safety**: Proper handling of international characters and emojis
- **Memory bounds checking**: Rust's safety guarantees prevent buffer overflows

The reduction process is highly optimized and adds minimal overhead to the extraction pipeline, typically processing documents in under 0.5ms regardless of the selected reduction mode.
