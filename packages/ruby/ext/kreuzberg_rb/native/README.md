# kreuzberg-rb

Magnus bindings for the Kreuzberg document intelligence library.

## Overview

This crate provides Ruby bindings to the Rust core library (`crates/kreuzberg`) using Magnus. It exposes extraction functions, configuration types, and plugin registration APIs to Ruby.

## Architecture

### Binding Layers

```
Ruby Package (packages/ruby/)
    ↓
Magnus Bindings (packages/ruby/ext/kreuzberg_rb/native) ← This crate
    ↓
Rust Core (crates/kreuzberg)
```

### Key Components

- **Core API** (`src/lib.rs`): Extraction functions (sync & async variants)
- **Configuration Parsing**: Ruby Hash to Rust config conversion
- **Type Conversion**: Rust results to Ruby hashes
- **Plugin Bridges**: Ruby plugin registration (PostProcessor, Validator, OcrBackend)
- **Cache Management**: Cache utilities

## Async Runtime Implementation

### Current State: Limited Async Support

Unlike NAPI-RS (Node.js) and PyO3 (Python), Magnus **does not have a pyo3-async-runtimes equivalent**. Ruby bindings use a different async pattern:

#### Async Functions Use Tokio Runtime with GVL Blocking

**Implementation** (from `src/lib.rs:584-602`):
```rust
fn extract_file(args: &[Value]) -> Result<RHash, Error> {
    let ruby = Ruby::get().expect("Ruby not initialized");
    let args = scan_args::<(String,), (Option<String>,), (), (), RHash, ()>(args)?;
    let (path,) = args.required;
    let (mime_type,) = args.optional;
    let opts = Some(args.keywords);

    let config = parse_extraction_config(&ruby, opts)?;

    // Use Tokio runtime to block on async function
    let runtime = tokio::runtime::Runtime::new()
        .map_err(|e| runtime_error(format!("Failed to create Tokio runtime: {}", e)))?;

    let result = runtime
        .block_on(async { kreuzberg::extract_file(&path, mime_type.as_deref(), &config).await })
        .map_err(kreuzberg_error)?;

    extraction_result_to_ruby(&ruby, result)
}
```

**What This Means**:
- ✅ **Works correctly** - Executes async Rust code successfully
- ⚠️ **Blocks Ruby thread** - Ruby thread frozen during async operations
- ❌ **No concurrency** - No performance benefit over synchronous calls from Ruby's perspective
- ❌ **GVL held** - Global VM Lock held during entire async operation

**Ruby Usage**:
```ruby
# This looks like it might be async, but it blocks the Ruby thread
result = Kreuzberg.extract_file("document.pdf")
```

### Why Magnus Differs from PyO3

| Feature | Magnus (Ruby) | PyO3 (Python) | NAPI-RS (Node.js) |
|---------|---------------|---------------|-------------------|
| Async Method Support | ❌ No | ✅ Yes | ✅ Yes |
| Runtime Integration | Manual `block_on()` | `pyo3_async_runtimes` | Built-in |
| GVL/GIL Release | ❌ Not available | ✅ Automatic | ✅ N/A (no GIL) |
| Coroutine Detection | ❌ N/A | ✅ `__await__` check | ✅ N/A (Promises) |
| Performance Optimization | ❌ Not possible | ✅ ~28x overhead reduction | ✅ ~0ms overhead |

### GVL Management

**Current State** (from Magnus maintainer matsadler):
> "Ruby does have a function to release the GVL called `rb_thread_call_without_gvl`, but it's hard to use correctly and Magnus doesn't expose it yet."

**Implications**:
- Async Rust operations **block the Ruby GVL**
- No concurrent Ruby execution during async Rust calls
- Performance similar to synchronous operations from Ruby's perspective

**Future Enhancement**: Safe `rb_thread_call_without_gvl` integration could enable:
- True concurrent async operations
- GVL release during Rust async waits
- Performance improvements for I/O-bound operations

### Comparison: Sync vs Async Functions

Both approaches currently have **equivalent performance** from Ruby's perspective:

**Synchronous Function**:
```rust
fn extract_file_sync(args: &[Value]) -> Result<RHash, Error> {
    let config = parse_extraction_config(&ruby, opts)?;
    let result = kreuzberg::extract_file_sync(&path, mime_type.as_deref(), &config)
        .map_err(kreuzberg_error)?;
    extraction_result_to_ruby(&ruby, result)
}
```

**Asynchronous Function**:
```rust
fn extract_file(args: &[Value]) -> Result<RHash, Error> {
    let config = parse_extraction_config(&ruby, opts)?;
    let runtime = tokio::runtime::Runtime::new()?;
    let result = runtime
        .block_on(async { kreuzberg::extract_file(&path, mime_type.as_deref(), &config).await })
        .map_err(kreuzberg_error)?;
    extraction_result_to_ruby(&ruby, result)
}
```

**Performance**: Both block Ruby thread for same duration. Use `_sync` variants for clarity.

## Plugin System

The bindings support Ruby-based plugins through the trait-based plugin system:

### Ruby PostProcessor Plugin

```ruby
Kreuzberg.register_post_processor("uppercase", ->(result) {
  result[:content] = result[:content].upcase
  result
}, 100)
```

**Implementation** (`src/lib.rs:831-939`):
- Wraps Ruby Proc in `RubyPostProcessor` struct
- Implements `PostProcessor` trait
- Marked `unsafe impl Send + Sync` (safe due to Ruby GVL)
- Converts Rust result → Ruby hash → calls Proc → converts back

### Ruby Validator Plugin

```ruby
Kreuzberg.register_validator("min_length", ->(result) {
  raise "Content too short" if result[:content].length < 100
}, 100)
```

**Implementation** (`src/lib.rs:954-1047`):
- Wraps Ruby Proc in `RubyValidator` struct
- Implements `Validator` trait
- Validates extraction results
- Can raise Ruby exceptions for validation failures

### Ruby OCR Backend Plugin

```ruby
class CustomOcr
  def process_image(image_bytes, language)
    # Return extracted text
    "Extracted text"
  end

  def supports_language?(lang)
    %w[eng deu fra].include?(lang)
  end
end

Kreuzberg.register_ocr_backend("custom", CustomOcr.new)
```

**Implementation** (`src/lib.rs:1070-1169`):
- Wraps Ruby object in `RubyOcrBackend` struct
- Implements `OcrBackend` trait
- Calls Ruby methods for OCR processing
- **Blocks GVL during OCR** (no async support)

**Note**: Ruby OCR backends will block the GVL during processing. For I/O-bound OCR operations, consider using Ruby threads or background jobs.

## Thread Safety

All Ruby plugin wrappers are marked `unsafe impl Send + Sync`:

```rust
// SAFETY: We mark this as Send+Sync because Ruby Global VM Lock (GVL)
// ensures thread safety. Magnus::Value is thread-safe under GVL.
struct RubyPostProcessor {
    name: String,
    processor: magnus::Value,
}

// SAFETY: Ruby operations are protected by the Global VM Lock
unsafe impl Send for RubyPostProcessor {}
unsafe impl Sync for RubyPostProcessor {}
```

**Justification**:
- Ruby's Global VM Lock ensures thread safety
- `magnus::Value` is thread-safe under GVL
- Rust async runtime can safely schedule Ruby callbacks
- GVL prevents concurrent Ruby execution

## Building

### Development Build

```bash
cd packages/ruby
bundle exec rake compile
```

### Testing

Run Ruby tests that exercise the bindings:

```bash
bundle exec rspec
```

## Features

### Default Features

- None currently

### Optional Features

All features are passed through from the `kreuzberg` crate via `features = ["full"]`.

## Dependencies

- `magnus` - Git dependency (specific rev: `f6db117`)
- `tokio = "1.48"` with `rt` and `macros` features
- `async-trait = "0.1"` for async trait methods
- `serde_json = "1.0"` for metadata serialization

## Key Files

- `src/lib.rs`: All bindings code (extraction API, config parsing, plugin registration)
- `build.rs`: Build script for Rust extension compilation

## References

- **Magnus Documentation**: https://docs.rs/magnus
- **Magnus GitHub**: https://github.com/matsadler/magnus
- **Kreuzberg Core**: `../kreuzberg/`
- **Ruby Package**: `../../packages/ruby/`

## Performance Considerations

### For Plugin Authors

1. **Sync is currently equivalent to async**: Both block the Ruby GVL
   ```ruby
   # These have equivalent performance
   result = Kreuzberg.extract_file_sync("document.pdf")
   result = Kreuzberg.extract_file("document.pdf")
   ```

2. **Use Ruby threads for concurrency**:
   ```ruby
   # Process multiple files concurrently
   files = ["doc1.pdf", "doc2.pdf", "doc3.pdf"]
   threads = files.map do |file|
     Thread.new { Kreuzberg.extract_file_sync(file) }
   end
   results = threads.map(&:value)
   ```

3. **Batch API is more efficient**:
   ```ruby
   # Prefer batch API for multiple files
   results = Kreuzberg.batch_extract_files_sync(["doc1.pdf", "doc2.pdf"])
   ```

### For Contributors

1. **Prefer `_sync` variants** - Clearer intent, same performance
2. **Async functions exist** for API compatibility with other language bindings
3. **Do NOT use `spawn_blocking`** - GVL already blocks, no benefit
4. **Create new Runtime per call** - Safe under GVL, no overhead from reuse
5. **Monitor Magnus development** for `rb_thread_call_without_gvl` exposure
6. **Thread safety via GVL** - `unsafe impl Send + Sync` is safe for Ruby callbacks

## Comparison with Other Language Bindings

### Async Support Ranking

1. **NAPI-RS (TypeScript/Node.js)**: ⭐⭐⭐⭐⭐
   - Built-in async support
   - Zero configuration
   - ~0ms overhead
   - Natural Promise integration

2. **PyO3 (Python)**: ⭐⭐⭐⭐
   - `pyo3_async_runtimes` library
   - Automatic async detection
   - ~0.17ms overhead (optimized)
   - GIL release during await

3. **Magnus (Ruby)**: ⭐⭐
   - Manual `block_on()` pattern
   - GVL blocks during async operations
   - Same overhead as sync
   - Limited concurrency

### When to Use Ruby Bindings

**Ruby bindings are best for**:
- ✅ **Rails applications** (ActiveJob for background processing)
- ✅ **Ruby scripts** (existing Ruby codebases)
- ✅ **Simple extraction** (single-file processing)
- ✅ **Batch processing** (batch API handles concurrency)

**Consider other bindings for**:
- ❌ **High concurrency** (use Node.js/NAPI-RS instead)
- ❌ **Real-time processing** (use Node.js/NAPI-RS instead)
- ❌ **I/O-bound workloads** (use Python/PyO3 or Node.js/NAPI-RS)

## GVL Release with Lucchetto (Experimental)

### Discovery: lucchetto Crate

**Lucchetto** (v0.4.0) is a third-party crate that enables calling Rust functions without holding the GVL:

```rust
use lucchetto::without_gvl;

#[without_gvl]
fn process_document(path: String) -> String {
    // GVL released during execution!
    // Other Ruby threads can run concurrently
    std::thread::sleep(std::time::Duration::from_secs(2));
    format!("Processed: {}", path)
}
```

**How it works**:
- Uses `rb_thread_call_without_gvl` internally (the hard-to-use-correctly function)
- Provides `#[without_gvl]` attribute macro for safe GVL release
- Enforces safety via `GvlSafe` trait (similar to `Send` + `Sync`)
- Functions can only accept/return types implementing `GvlSafe`

**Dependencies**:
```toml
[dependencies]
lucchetto = "0.4.0"
lucchetto-macros = "0.2.0"
rb-sys = "0"
```

**Safety Model**:
- `GvlSafe` trait prevents accessing Ruby objects from GVL-free code
- Custom types can implement `GvlSafe` if they don't interact with Ruby VM
- Compile-time verification via trait bounds

**Limitations**:
- ⚠️ **Experimental**: Author notes potential memory bugs and unsafe code
- ⚠️ **0.4.0 version**: Early stage, API may change
- ⚠️ **Documentation**: 0% coverage, review source code before use
- ⚠️ **Cannot access Ruby objects** during GVL-free execution

**Potential Integration**:

```rust
use lucchetto::without_gvl;

// Long-running extraction that doesn't need Ruby access
#[without_gvl]
fn extract_large_pdf_internal(path: String) -> Result<String, String> {
    // GVL released - other Ruby threads can run!
    match kreuzberg::extract_file_sync(&path, None, &Default::default()) {
        Ok(result) => Ok(result.content),
        Err(e) => Err(e.to_string()),
    }
}

// Ruby-facing wrapper
fn extract_file_sync(args: &[Value]) -> Result<RHash, Error> {
    let ruby = Ruby::get().expect("Ruby not initialized");
    let path: String = args.get(0).unwrap().try_convert()?;

    // Call GVL-free function
    let content = extract_large_pdf_internal(path)
        .map_err(|e| runtime_error(e))?;

    // Convert to Ruby (GVL held)
    let hash = ruby.hash_new();
    hash.aset(ruby.intern("content"), content)?;
    Ok(hash)
}
```

**Performance Impact**:
- ✅ **Enables true concurrency**: Ruby threads can run during Rust operations
- ✅ **No GVL blocking**: Long operations don't freeze Ruby runtime
- ✅ **Thread-level parallelism**: Multiple Ruby threads can process different files

**Recommendation**:
- **Monitor lucchetto development** before production use
- **Test thoroughly** in development environment
- **Consider for CPU-bound operations** (PDF extraction, OCR, image processing)
- **Not recommended yet** for production due to experimental status

## Future Improvements

Potential areas for async enhancement:

1. **Lucchetto integration** - Evaluate for GVL-free extraction (experimental)
2. **Ruby Fiber integration** - Map Rust futures to Ruby Fibers
3. **Async OCR backends** - Non-blocking OCR processing with GVL release
4. **Streaming results** - Chunked extraction without blocking GVL

**Contributing**: If you're interested in improving async support, check:
- Lucchetto crate: https://github.com/Maaarcocr/lucchetto
- Magnus GitHub Issues: https://github.com/matsadler/magnus/issues
- `rb_thread_call_without_gvl` discussions
- Ruby Fiber-based async patterns

## Contributing

See the main Kreuzberg repository for contribution guidelines.
