# Caching System

Kreuzberg includes an intelligent caching system that speeds up repeated document processing operations.

## Overview

The caching system stores results from expensive operations to avoid redundant computation:

- **OCR results**: Cached text extraction from images
- **Embeddings**: Cached vector representations for RAG systems
- **Extraction results**: Cached processed documents

## How It Works

### Cache Strategy

Kreuzberg uses a **content-addressable cache** based on input characteristics:

1. **Cache Key Generation**: Deterministic keys based on input content and configuration
2. **Validation**: Checks if cached results are still valid (source file unchanged)
3. **Smart Cleanup**: Automatic eviction based on age, size, and disk space
4. **Concurrency Control**: Thread-safe operations prevent race conditions

### Cache Invalidation

Cached results are automatically invalidated when:

- Source file has been modified (size or timestamp changed)
- Cache entry exceeds maximum age
- Cache size exceeds configured limits
- Available disk space falls below threshold

### Storage

Cache is stored in `~/.cache/kreuzberg/` with:

- **Binary serialization**: Compact MessagePack format for efficiency
- **Metadata tracking**: Source file size and modification time
- **LRU eviction**: Oldest entries removed first when cleanup needed

## Configuration

Cache behavior is controlled through extraction configuration:

=== "Python"

    ```python
    from kreuzberg import extract_file_sync, ExtractionConfig

    config = ExtractionConfig(
        use_cache=True,  # Enable caching (default: True)
    )

    result = extract_file_sync("document.pdf", config=config)
    ```

=== "TypeScript"

    ```typescript
    import { extractFileSync, ExtractionConfig } from '@goldziher/kreuzberg';

    const config = new ExtractionConfig({
        useCache: true,  // Enable caching (default: true)
    });

    const result = extractFileSync('document.pdf', null, config);
    ```

=== "Rust"

    ```rust
    use kreuzberg::{extract_file_sync, ExtractionConfig};

    let config = ExtractionConfig {
        use_cache: true,
        ..Default::default()
    };

    let result = extract_file_sync("document.pdf", None, &config)?;
    ```

=== "Ruby"

    ```ruby
    require 'kreuzberg'

    config = Kreuzberg::Config::Extraction.new(use_cache: true)

    result = Kreuzberg.extract_file_sync('document.pdf', config: config)
    ```

## Performance Impact

Caching provides significant performance improvements for repeated operations:

- **OCR operations**: Avoid re-processing identical images
- **Embedding generation**: Reuse previously computed vectors
- **Batch processing**: Speed up processing similar documents

Cache hits return results instantly, eliminating expensive computation.

## Cache Management

### Automatic Cleanup

The cache automatically manages disk space:

- Removes entries older than configured age
- Evicts oldest entries when size limit exceeded
- Frees space when disk becomes full

### Manual Clearing

Clear cache programmatically when needed:

=== "Python"

    ```python
    from kreuzberg._utils._cache import clear_cache

    clear_cache()  # Clear all cached results
    ```

=== "TypeScript"

    ```typescript
    // Manual clearing: rm -rf ~/.cache/kreuzberg/
    ```

=== "Rust"

    ```rust
    use kreuzberg::cache::GenericCache;

    let cache = GenericCache::new("ocr", None, 30.0, 500.0, 1000.0)?;
    cache.clear()?;
    ```

=== "Ruby"

    ```ruby
    require 'kreuzberg'

    Kreuzberg.clear_cache
    ```

## Best Practices

- **Enable caching for production**: Significant performance gains for repeated operations
- **Disable for one-off processing**: Skip cache overhead for unique documents
- **Monitor cache size**: Ensure adequate disk space for your workload
- **Clear cache after major updates**: Ensure compatibility with new versions

## Next Steps

- [Architecture](architecture.md) - System design overview
- [OCR System](ocr.md) - OCR caching details
