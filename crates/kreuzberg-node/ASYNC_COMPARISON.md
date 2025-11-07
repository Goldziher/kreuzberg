# Async Patterns: NAPI-RS vs PyO3 vs Magnus

**Date**: 2025-01-11
**Purpose**: Compare async runtime integration across Node.js (NAPI-RS), Python (PyO3), and Ruby (Magnus) bindings

## Executive Summary

**NAPI-RS (Node.js) provides superior out-of-the-box async support. PyO3 (Python) has excellent optimized support. Magnus (Ruby) has limited async support.**

| Feature | NAPI-RS (Node.js) | PyO3 (Python) | Magnus (Ruby) |
|---------|-------------------|---------------|---------------|
| Async Support | ✅ Built-in | ⚠️ Requires pyo3-async-runtimes | ❌ block_on() only |
| API Complexity | ⭐ Simple (`async fn`) | ⭐⭐ Manual (detection + conversion) | ⭐⭐⭐ Manual (no async benefit) |
| Runtime Init | ✅ Automatic | ⚠️ Manual with OnceCell | ⚠️ Per-call runtime creation |
| Overhead | 🚀 ~0ms (transparent) | 🚀 ~0.17ms (optimized) | ❌ Same as sync (GVL blocks) |
| GIL/GVL Release | ✅ N/A (no GIL) | ✅ Automatic | ❌ Not available |
| Developer Experience | ⭐⭐⭐ Excellent | ⭐⭐ Good | ⭐ Limited |

## Implementation Comparison

### NAPI-RS (Node.js) - Built-in Async Support

**Implementation** (simple):
```rust
#[napi]
pub async fn extract_file(
    file_path: String,
    mime_type: Option<String>,
    config: Option<JsExtractionConfig>,
) -> Result<JsExtractionResult> {
    let rust_config = config.map(Into::into).unwrap_or_default();

    kreuzberg::extract_file(&file_path, mime_type.as_deref(), &rust_config)
        .await
        .map_err(convert_error)
        .and_then(JsExtractionResult::try_from)
}
```

**That's it!** NAPI-RS automatically:
- Executes Tokio futures on the Tokio runtime
- Converts results to JavaScript Promises
- Handles all FFI bridging transparently

**JavaScript Usage**:
```javascript
// Simple async/await
const result = await extractFile('document.pdf', null, null);

// Or Promise chaining
extractFile('document.pdf', null, null)
    .then(result => console.log(result.content))
    .catch(err => console.error(err));
```

### PyO3 (Python) - Requires pyo3-async-runtimes

**Implementation** (more complex):
```rust
// 1. Initialize event loop once
use once_cell::sync::OnceCell;
use pyo3_async_runtimes::TaskLocals;

static TASK_LOCALS: OnceCell<TaskLocals> = OnceCell::new();

#[pyfunction]
fn init_async_runtime() -> PyResult<()> {
    Python::attach(|py| {
        let asyncio = py.import("asyncio")?;
        let event_loop = asyncio.call_method0("new_event_loop")?;
        TASK_LOCALS.get_or_init(|| TaskLocals::new(event_loop.into()));
        Ok(())
    })
}

// 2. Export Rust async function to Python
#[pyfunction]
fn extract_file<'py>(
    py: Python<'py>,
    path: String,
    mime_type: Option<String>,
    config: ExtractionConfig,
) -> PyResult<Bound<'py, PyAny>> {
    let rust_config: kreuzberg::ExtractionConfig = config.into();

    // Manual conversion of Rust future to Python awaitable
    pyo3_async_runtimes::tokio::future_into_py(py, async move {
        let result = kreuzberg::extract_file(&path, mime_type.as_deref(), &rust_config)
            .await
            .map_err(to_py_err)?;
        Python::attach(|py| ExtractionResult::from_rust(result, py))
    })
}
```

**Python Usage**:
```python
# Must initialize runtime first
from kreuzberg._internal_bindings import init_async_runtime
init_async_runtime()

# Then use async functions
result = await extract_file('document.pdf', None, config)
```

### Magnus (Ruby) - Limited Async Support (with Lucchetto Option)

**Current Implementation** (blocks GVL):
```rust
fn extract_file(args: &[Value]) -> Result<RHash, Error> {
    let ruby = Ruby::get().expect("Ruby not initialized");
    let args = scan_args::<(String,), (Option<String>,), (), (), RHash, ()>(args)?;
    let (path,) = args.required;
    let (mime_type,) = args.optional;
    let opts = Some(args.keywords);

    let config = parse_extraction_config(&ruby, opts)?;

    // Use Tokio runtime to block on async function
    // Blocks Ruby GVL during entire async operation
    let runtime = tokio::runtime::Runtime::new()
        .map_err(|e| runtime_error(format!("Failed to create Tokio runtime: {}", e)))?;

    let result = runtime
        .block_on(async { kreuzberg::extract_file(&path, mime_type.as_deref(), &config).await })
        .map_err(kreuzberg_error)?;

    extraction_result_to_ruby(&ruby, result)
}
```

**Ruby Usage**:
```ruby
# Looks async but blocks the Ruby thread (GVL held)
result = Kreuzberg.extract_file("document.pdf")

# Synchronous variant has identical performance
result = Kreuzberg.extract_file_sync("document.pdf")

# For concurrency, use Ruby threads
threads = ["doc1.pdf", "doc2.pdf"].map do |file|
  Thread.new { Kreuzberg.extract_file_sync(file) }
end
results = threads.map(&:value)
```

**Experimental: GVL Release with Lucchetto**:
```rust
use lucchetto::without_gvl;

// Long-running function that doesn't access Ruby objects
#[without_gvl]
fn extract_large_pdf_internal(path: String) -> Result<String, String> {
    // GVL released - other Ruby threads can run concurrently!
    match kreuzberg::extract_file_sync(&path, None, &Default::default()) {
        Ok(result) => Ok(result.content),
        Err(e) => Err(e.to_string()),
    }
}

// Ruby-facing wrapper
fn extract_file_sync(args: &[Value]) -> Result<RHash, Error> {
    let ruby = Ruby::get().expect("Ruby not initialized");
    let path: String = args.get(0).unwrap().try_convert()?;

    // Call GVL-free function - other threads run during extraction
    let content = extract_large_pdf_internal(path)
        .map_err(|e| runtime_error(e))?;

    // Convert to Ruby (GVL held again)
    let hash = ruby.hash_new();
    hash.aset(ruby.intern("content"), content)?;
    Ok(hash)
}
```

**Lucchetto Notes**:
- ⚠️ **Experimental** (v0.4.0) - Author notes potential memory bugs
- ✅ **Enables true concurrency** - Ruby threads can run during Rust operations
- ✅ **GvlSafe trait** - Compile-time safety for types
- ❌ **Cannot access Ruby objects** during GVL-free execution
- **Not recommended for production** until more mature

## Feature Comparison

### 1. Async Function Declaration

**NAPI-RS**:
```rust
#[napi]
pub async fn function_name(...) -> Result<T> {
    // Just write async Rust
}
```

**PyO3**:
```rust
#[pyfunction]
fn function_name<'py>(py: Python<'py>, ...) -> PyResult<Bound<'py, PyAny>> {
    pyo3_async_runtimes::tokio::future_into_py(py, async move {
        // Async Rust in closure
    })
}
```

**Magnus**:
```rust
fn function_name(args: &[Value]) -> Result<RHash, Error> {
    let runtime = tokio::runtime::Runtime::new()?;
    let result = runtime.block_on(async {
        // Async Rust that blocks GVL
    })?;
    // Convert to Ruby
}
```

### 2. Promise/Coroutine Conversion

**NAPI-RS**: ✅ Automatic
**PyO3**: ⚠️ Manual via `future_into_py()`
**Magnus**: ❌ N/A - blocks thread, no conversion

### 3. Runtime Management

**NAPI-RS**: ✅ Handled by framework
**PyO3**: ⚠️ Manual `OnceCell` + `TaskLocals`
**Magnus**: ⚠️ New runtime per call (GVL blocks reuse benefit)

### 4. Event Loop Initialization

**NAPI-RS**: ✅ None needed
**PyO3**: ⚠️ Must call `init_async_runtime()`
**Magnus**: ⚠️ Runtime created per call

### 5. Async Detection (for plugins)

**NAPI-RS**: ✅ N/A - JavaScript Promises are always async
**PyO3**: ⚠️ Must check `__await__` attribute
**Magnus**: ❌ N/A - Ruby doesn't have native async

### 6. GIL/GVL Management

**NAPI-RS**: ✅ No GIL - Node.js uses event loop
**PyO3**: ✅ GIL released during await
**Magnus**: ❌ GVL held during entire async operation

## Performance Characteristics

### NAPI-RS (Node.js)

| Metric | Value | Notes |
|--------|-------|-------|
| Async overhead | ~0ms | Transparent Tokio integration |
| Promise creation | Instant | Built into framework |
| Runtime init | None | Automatic |
| Concurrency model | Event loop | Non-blocking by default |

### PyO3 (Python)

| Metric | Without pyo3-async-runtimes | With pyo3-async-runtimes |
|--------|----------------------------|-------------------------|
| Async overhead | ~4.8ms (spawn_blocking) | ~0.17ms (into_future) |
| Coroutine creation | N/A | ~0.05ms |
| Runtime init | Once at startup | Once at startup |
| Event loop overhead | ~55µs per call | Eliminated (OnceCell) |
| Concurrency model | GIL-limited | GIL released during await |

**Speedup with pyo3-async-runtimes**: ~28x overhead reduction

### Magnus (Ruby)

| Metric | Value | Notes |
|--------|-------|-------|
| Async overhead | Same as sync | GVL blocks entire operation |
| Runtime creation | Per call | ~negligible overhead |
| Concurrency model | GVL-blocked | No true async concurrency |
| Thread safety | GVL-protected | Safe for multi-threading |
| Async benefit | None | Use `_sync` variants instead |

**Performance**: Async and sync functions have identical performance from Ruby's perspective

## Code Complexity Comparison

### Lines of Code

**NAPI-RS** (async fn export):
```
3-5 lines per function (just the function signature and body)
```

**PyO3** (async fn export):
```
~15-20 lines per function:
- Function signature with Python lifetime
- future_into_py wrapper
- async move closure
- Result conversion
- Error mapping
```

**PyO3** (runtime setup):
```
~40 additional lines:
- OnceCell static declaration
- init_async_runtime() function
- get_task_locals() helper
- Python module registration
```

**Magnus** (async fn export):
```
~10-15 lines per function:
- Function signature with Ruby Value args
- Runtime creation (per call)
- block_on() wrapper
- async move closure
- Result conversion to Ruby Hash
```

**Magnus** (no runtime setup needed):
```
0 additional lines - runtime created per call
(GVL blocks any reuse benefit)
```

### Developer Experience

**NAPI-RS**:
1. Write `#[napi] pub async fn`
2. Done ✅

**PyO3**:
1. Set up OnceCell + TaskLocals
2. Write init_async_runtime()
3. Wrap functions in future_into_py()
4. Handle Python lifetimes
5. Convert results
6. Expose init function to Python
7. Call init_async_runtime() in Python

**Magnus**:
1. Create Tokio runtime
2. Wrap async code in block_on()
3. Convert result to Ruby Hash
4. Accept that it blocks GVL (no async benefit)

## Real-World Examples

### NAPI-RS: Concurrent File Processing

```javascript
// Natural JavaScript async/await
const files = ['doc1.pdf', 'doc2.pdf', 'doc3.pdf'];

// Process concurrently with Promise.all
const results = await Promise.all(
    files.map(file => extractFile(file, null, null))
);

// Or use built-in batch function
const results = await batchExtractFiles(files, null);
```

### PyO3: Same Task

```python
import asyncio
from kreuzberg import extract_file
from kreuzberg._internal_bindings import init_async_runtime

# Must initialize first
init_async_runtime()

files = ['doc1.pdf', 'doc2.pdf', 'doc3.pdf']

# Process concurrently with gather
results = await asyncio.gather(*[
    extract_file(file, None, None) for file in files
])

# Or use built-in batch function
results = await batch_extract_files(files, None)
```

### Magnus: Same Task

```ruby
require 'kreuzberg'

files = ['doc1.pdf', 'doc2.pdf', 'doc3.pdf']

# Option 1: Use batch API (most efficient - Rust handles concurrency)
results = Kreuzberg.batch_extract_files_sync(files)

# Option 2: Use Ruby threads for concurrency
threads = files.map do |file|
  Thread.new { Kreuzberg.extract_file_sync(file) }
end
results = threads.map(&:value)

# Option 3: Sequential (simplest but slowest)
results = files.map { |file| Kreuzberg.extract_file_sync(file) }
```

## When Each Shines

### NAPI-RS is Best For:

- ✅ **Web servers** (Express, Fastify, Nest.js)
- ✅ **Serverless functions** (AWS Lambda, Vercel)
- ✅ **Real-time applications** (WebSocket servers)
- ✅ **CLI tools** with async I/O
- ✅ **Build tools** and bundlers

**Why**: Event-driven architecture, non-blocking by default, excellent async support

### PyO3 is Best For:

- ✅ **Data science pipelines** (pandas, numpy integration)
- ✅ **ML/AI workflows** (PyTorch, TensorFlow)
- ✅ **Scientific computing** (scipy, scikit-learn)
- ✅ **Notebooks** (Jupyter)
- ✅ **Legacy Python codebases**

**Why**: Rich ecosystem, easy integration with existing Python libraries

### Magnus is Best For:

- ✅ **Rails applications** (ActiveJob for background processing)
- ✅ **Ruby scripts** (existing Ruby codebases)
- ✅ **Simple extraction** (single-file processing)
- ✅ **Batch processing** (batch API handles concurrency internally)
- ✅ **Integration with Ruby gems**

**Why**: Native Ruby integration, familiar syntax, GVL-protected thread safety

**Not Recommended For**:
- ❌ High concurrency workloads (use Node.js/NAPI-RS)
- ❌ Real-time processing (use Node.js/NAPI-RS)
- ❌ I/O-bound async operations (use Python/PyO3 or Node.js/NAPI-RS)

## Conclusion

### Async Support Ranking

1. **NAPI-RS (TypeScript/Node.js)**: ⭐⭐⭐⭐⭐ Excellent
   - Zero configuration, built-in support
   - ~0ms overhead, transparent integration
   - Best developer experience

2. **PyO3 (Python)**: ⭐⭐⭐⭐ Very Good
   - Requires pyo3-async-runtimes library
   - ~0.17ms overhead (optimized)
   - Excellent performance after setup

3. **Magnus (Ruby)**: ⭐⭐ Limited
   - No true async support
   - GVL blocks during operations
   - Same performance as sync

### Recommendations by Use Case

**For async/concurrent workloads**:
- **Best**: NAPI-RS (Node.js) - simplest and most performant
- **Good**: PyO3 (Python) - excellent performance with setup
- **Limited**: Magnus (Ruby) - use batch API or Ruby threads

**For ecosystem and integration**:
- **Python**: Rich data science/ML ecosystem (pandas, PyTorch, NumPy)
- **Node.js**: Web development, real-time apps, serverless
- **Ruby**: Rails applications, background jobs (ActiveJob, Sidekiq)

**For simplicity**:
- **Node.js**: Zero config async
- **Ruby**: Familiar Ruby syntax, GVL-protected safety
- **Python**: Requires runtime setup but auto-detects async

### Final Recommendations

**Choose Node.js/NAPI-RS for**:
- Web APIs, servers, real-time applications
- High concurrency requirements
- Serverless functions
- Best async developer experience

**Choose Python/PyO3 for**:
- Data science, ML, scientific computing
- Rich Python ecosystem integration
- I/O-bound async workloads
- Excellent async performance

**Choose Ruby/Magnus for**:
- Rails applications
- Existing Ruby codebases
- Simple extraction tasks
- Batch processing (batch API recommended)

## References

- **NAPI-RS Async Documentation**: https://napi.rs/docs/concepts/async-fn
- **pyo3-async-runtimes**: https://docs.rs/pyo3-async-runtimes
- **Magnus Documentation**: https://docs.rs/magnus
- **Lucchetto (GVL Release)**: https://github.com/Maaarcocr/lucchetto
- **Lucchetto Documentation**: https://docs.rs/lucchetto
- **Research Document**: `docs/ASYNC_RUNTIME_RESEARCH.md`
- **Python Bindings README**: `crates/kreuzberg-py/README.md`
- **Ruby Bindings README**: `crates/kreuzberg-rb/README.md`
- **Node.js Examples**: `crates/kreuzberg-node/examples/async-patterns.ts`
- **Ruby Examples**: `packages/ruby/examples/async_patterns.rb`
