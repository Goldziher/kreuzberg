# Performance Guide

Kreuzberg provides both synchronous and asynchronous APIs, each optimized for different use cases. This guide helps you choose the right approach and understand the performance characteristics.

## Quick Reference

| Use Case            | Recommended API              | Reason                                 |
| ------------------- | ---------------------------- | -------------------------------------- |
| CLI tools           | `extract_file_sync()`        | Lower overhead, simpler code           |
| **Backend APIs**    | `await extract_file()`       | **Always use async in async contexts** |
| Web applications    | `await extract_file()`       | Better concurrency                     |
| Simple documents    | `extract_file_sync()`        | Faster for small files                 |
| Complex PDFs        | `await extract_file()`       | Parallelized processing                |
| Batch processing    | `await batch_extract_file()` | Concurrent execution                   |
| OCR-heavy workloads | `await extract_file()`       | Multiprocessing benefits               |

## Competitive Performance

[Live benchmarks](https://benchmarks.kreuzberg.dev/) ([source code](https://github.com/Goldziher/python-text-extraction-libs-benchmarks)) demonstrate Kreuzberg as the fastest Python CPU-based text extraction framework:

- **Leading Performance**: 31.78 files/second for small documents, 2.42 files/second for medium files
- **Minimal Memory**: ~360MB average usage, lowest among tested frameworks
- **Smallest Installation**: 71MB package size for maximum deployment flexibility
- **High Reliability**: 100% success rate across all 18 tested file formats
- **Production Optimized**: Built for high-throughput, real-time applications

## Internal Benchmark Results

All internal benchmarks were conducted on macOS 15.5 with ARM64 (14 cores, 48GB RAM) using Python 3.13.3.

### Single Document Processing

| Document Type        | File Size | Sync Time | Async Time | Speedup                  | Notes                    |
| -------------------- | --------- | --------- | ---------- | ------------------------ | ------------------------ |
| Markdown             | \<1KB     | 0.4ms     | 17.5ms     | **❌ Async 41x slower**  | Async overhead dominates |
| HTML                 | ~1KB      | 1.6ms     | 1.1ms      | **✅ Async 1.5x faster** | Minimal parsing overhead |
| PDF (searchable)     | ~10KB     | 3.4ms     | 2.7ms      | **✅ Async 1.3x faster** | Text extraction only     |
| PDF (non-searchable) | ~100KB    | 394ms     | 652ms      | **✅ Sync 1.7x faster**  | OCR processing           |
| PDF (complex)        | ~1MB      | 39.0s     | 8.5s       | **✅ Async 4.6x faster** | Heavy OCR + processing   |

### Batch Processing

| Operation        | Documents | Sync Time | Async Time | Speedup                  | Notes                     |
| ---------------- | --------- | --------- | ---------- | ------------------------ | ------------------------- |
| Sequential batch | 3 mixed   | 38.6s     | N/A        | N/A                      | Sync processes one by one |
| Concurrent batch | 3 mixed   | N/A       | 8.5s       | **✅ Async 4.5x faster** | Parallel processing       |

## Performance Analysis

### Why Async Wins for Complex Tasks

1. **Multiprocessing**: Async implementation uses multiprocessing for CPU-intensive OCR
1. **Concurrency**: I/O operations don't block other tasks
1. **Resource Management**: Better memory and CPU utilization
1. **Parallel OCR**: Multiple pages processed simultaneously

### Why Sync Wins for Simple Tasks

1. **No Overhead**: Direct function calls without async/await machinery
1. **Lower Memory**: No event loop or task scheduling overhead
1. **Simpler Path**: Direct execution without thread/process coordination
1. **Fast Startup**: Immediate execution for quick operations

### Backend API Considerations

**Important**: When working in an async context (like FastAPI, Django async views, aiohttp), **always use the async API** even for simple documents:

```python
# ✅ Correct: Use async in async contexts
async def extract_endpoint(file_path: str):
    result = await extract_file(file_path)  # Non-blocking
    return result

# ❌ Wrong: Sync in async context blocks the event loop
async def extract_endpoint(file_path: str):
    result = extract_file_sync(file_path)  # Blocks event loop!
    return result
```

**Why this matters:**

- Sync operations in async contexts block the entire event loop
- This prevents other requests from being processed concurrently
- Backend throughput drops dramatically
- Use async consistently throughout your async application stack

### The Crossover Point

The performance crossover occurs around **10KB file size** or when **OCR is required**:

```python
# Use sync for simple cases
if file_size < 10_000 and not requires_ocr:
    result = extract_file_sync(path)
else:
    result = await extract_file(path)
```

## Implementation Details

### Synchronous Implementation

The sync API uses a pure synchronous multiprocessing approach:

- **Direct execution**: No async overhead
- **Process pools**: For CPU-intensive tasks like OCR
- **Memory efficient**: Lower baseline memory usage
- **Simple debugging**: Easier to profile and debug

### Asynchronous Implementation

The async API leverages Python's asyncio with intelligent task scheduling:

- **Event loop integration**: Non-blocking I/O operations
- **Concurrent processing**: Multiple documents simultaneously
- **Adaptive multiprocessing**: Dynamic worker allocation
- **Resource management**: Automatic cleanup and optimization

## Optimization Strategies

### For Maximum Performance

1. **Choose the right API** based on your use case
1. **Use batch operations** for multiple files
1. **Configure OCR appropriately** for your document types
1. **Profile your specific workload** - results vary by content

### Optimized Default Configuration

Kreuzberg's default configuration is **optimized out-of-the-box for modern PDFs and standard documents**:

```python
from kreuzberg import ExtractionConfig

# Default configuration - already optimized for modern documents
config = ExtractionConfig()  # Uses optimized defaults:
# - PSM: AUTO_ONLY (fast without orientation detection)
# - Language model: Disabled for performance
# - Dictionary correction: Enabled for accuracy
```

### Advanced Configuration Examples

```python
from kreuzberg import ExtractionConfig, extract_file_sync
from kreuzberg._ocr._tesseract import TesseractConfig, PSMMode

# Maximum speed configuration (for high-volume processing)
speed_config = ExtractionConfig(
    ocr_backend="tesseract",
    ocr_config=TesseractConfig(
        psm=PSMMode.SINGLE_BLOCK,  # Assume simple layout
        language_model_ngram_on=False,  # Already disabled by default
        tessedit_enable_dict_correction=False,  # Disable for maximum speed
    ),
)

# Maximum accuracy configuration (for degraded documents)
accuracy_config = ExtractionConfig(
    ocr_backend="tesseract",
    ocr_config=TesseractConfig(
        psm=PSMMode.AUTO,  # Full analysis with orientation detection
        language_model_ngram_on=True,  # Enable for historical/degraded text
        tessedit_enable_dict_correction=True,  # Default - keep enabled
    ),
)

# No OCR configuration (text documents only)
text_only_config = ExtractionConfig(ocr_backend=None, force_ocr=False)
```

### Performance Optimization Tips

Based on comprehensive benchmarking with 138+ documents:

1. **Disable OCR for text documents**: Setting `ocr_backend=None` provides significant speedup for documents with text layers
1. **Use PSM `AUTO_ONLY` (default)**: Optimized for modern documents without orientation detection overhead
1. **Language model trade-offs**: Disabling `language_model_ngram_on` can provide 30x+ speedup with minimal quality impact on clean documents
1. **Dictionary correction**: Disabling `tessedit_enable_dict_correction` speeds up processing for technical documents

### Batch Processing Best Practices

```python
import asyncio
from kreuzberg import batch_extract_file

async def process_many_files(file_paths):
    # Efficient: processes files concurrently
    results = await batch_extract_file(file_paths)
    return results

# Alternative: fine-grained control
async def process_with_control(file_paths):
    tasks = [extract_file(path) for path in file_paths]
    results = await asyncio.gather(*tasks, return_exceptions=True)
    return results
```

## Memory Usage

### Sync Memory Profile

- **Low baseline**: ~10-50MB for most operations
- **Predictable**: Memory usage scales linearly with file size
- **Fast cleanup**: Immediate garbage collection

### Async Memory Profile

- **Higher baseline**: ~50-100MB due to event loop overhead
- **Better scaling**: More efficient for large batches
- **Managed cleanup**: Automatic resource management

## Benchmarking Your Workload

To benchmark your specific use case, use our comprehensive benchmark suite:

```bash
# Clone the repository
git clone https://github.com/Goldziher/kreuzberg.git
cd kreuzberg

# Install with benchmark dependencies
uv sync --all-packages --all-extras --all-groups

# Run comprehensive benchmarks
cd benchmarks
uv run kreuzberg-bench run --comparison-only

# Or use the simple script for quick tests
cd ..
python scripts/run_benchmarks.py
```

### Benchmark Methodology

Our benchmarks follow rigorous methodology to ensure accurate results:

1. **Controlled Environment**: Tests run on dedicated CI infrastructure
1. **Multiple Iterations**: Each test runs multiple times for statistical significance
1. **Memory Monitoring**: Peak memory usage tracked throughout execution
1. **CPU Profiling**: Average CPU utilization measured
1. **Warm-up Runs**: JIT compilation effects minimized with warm-up iterations
1. **System Info Collection**: Hardware specs recorded for context

### Interpreting Results

- **Duration**: Lower is better, measured in seconds/milliseconds
- **Memory Peak**: Peak memory usage during operation (MB)
- **CPU Average**: Average CPU utilization percentage during test
- **Success Rate**: Percentage of benchmark runs that completed successfully

The benchmarks use real-world documents of varying complexity to simulate actual usage patterns.

For complete benchmark suite documentation, methodology details, and CI integration, see the [Benchmark Suite README](https://github.com/Goldziher/kreuzberg/tree/main/benchmarks).

## Troubleshooting Performance

### Common Issues

1. **Async slower than expected**: Check if async overhead dominates (small files)
1. **High memory usage**: Consider batch size and file types
1. **Slow OCR**: Verify OCR engine configuration and document quality
1. **CPU bottlenecks**: Monitor process pool utilization

### Profiling Tools

```python
import time
import psutil
from kreuzberg import extract_file_sync

def profile_extraction(file_path):
    process = psutil.Process()
    start_memory = process.memory_info().rss / 1024 / 1024

    start_time = time.perf_counter()
    result = extract_file_sync(file_path)
    duration = time.perf_counter() - start_time

    end_memory = process.memory_info().rss / 1024 / 1024

    print(f"Duration: {duration:.3f}s")
    print(f"Memory used: {end_memory - start_memory:.1f}MB")
    print(f"Content length: {len(result.content)}")
```

## Conclusion

Choose your API based on your specific needs:

- **Sync for simplicity**: CLI tools, simple documents, single-threaded applications
- **Async for scale**: Web applications, batch processing, complex documents
- **Async for backends**: **Always use async in async contexts** (FastAPI, Django async, etc.)
- **Batch for efficiency**: Multiple files, concurrent processing requirements

### Key Decision Points

1. **Are you in an async context?** → Use async API
1. **Processing multiple files?** → Use batch operations
1. **Simple single document in sync context?** → Sync may be faster
1. **Complex documents or OCR required?** → Use async API
1. **Building a web API?** → Use async API

The performance characteristics will vary based on your specific documents, hardware, and usage patterns. We recommend benchmarking with your actual data to make informed decisions.

**Remember**: Kreuzberg is benchmarked as one of the fastest text extraction libraries available, delivering superior performance regardless of which API you choose.
