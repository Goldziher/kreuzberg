# kreuzberg-py

PyO3 bindings for the Kreuzberg document intelligence library.

## Overview

This crate provides Python bindings to the Rust core library (`crates/kreuzberg`) using PyO3 0.27. It exposes extraction functions, configuration types, and plugin registration APIs to Python.

## Architecture

### Binding Layers

```
Python Package (packages/python/kreuzberg/)
    ↓
PyO3 Bindings (crates/kreuzberg-py) ← This crate
    ↓
Rust Core (crates/kreuzberg)
```

### Key Components

- **Core API** (`src/core.rs`): Extraction functions (sync & async)
- **Configuration** (`src/config.rs`): Config types with PyO3 wrappers
- **Plugins** (`src/plugins.rs`): Python plugin registration and bridges
- **Types** (`src/types.rs`): Result types and data structures
- **Errors** (`src/error.rs`): Exception types

## Performance Optimizations

### 1. Async Python Plugin Support (pyo3_async_runtimes)

The bindings use `pyo3_async_runtimes` for high-performance async Python callbacks, based on patterns from [spikard](https://github.com/Goldziher/spikard).

#### Pattern: spawn_blocking vs into_future

**Traditional pattern (slow)**:
```rust
tokio::task::spawn_blocking(move || {
    Python::attach(|py| {
        // Call Python function
    })
}).await?
// Overhead: ~4.8ms per call
```

**Optimized pattern (fast)**:
```rust
// Check if Python function is async
let is_async = Python::attach(|py| {
    obj.getattr("method")?.hasattr("__await__")
});

if is_async {
    // Convert Python coroutine → Rust future (no spawn_blocking!)
    let result = Python::attach(|py| {
        let coroutine = obj.call_method(...)?;
        pyo3_async_runtimes::tokio::into_future(coroutine)
    })?
    .await?;  // GIL released during await
    // Overhead: ~0.17ms per call (~28x faster)
} else {
    // Fallback for sync functions
    tokio::task::spawn_blocking(move || { ... }).await?
}
```

**Performance impact**:
- Fast operations (<10ms): ~25-30x speedup
- Medium operations (50ms): ~10% speedup
- Overhead reduction: ~4.8ms → ~0.17ms

#### Event Loop Reuse

Initialize Python event loop once to avoid ~55µs overhead per call:

```rust
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
```

**Usage from Python**:
```python
from kreuzberg._internal_bindings import init_async_runtime
init_async_runtime()  # Call once at startup
```

#### Automatic Detection

The bindings automatically detect async Python functions and use the optimized path:

```python
class AsyncOcrBackend:
    async def process_image(self, image_bytes: bytes, language: str) -> dict:
        # Automatically uses pyo3_async_runtimes (fast path)
        await asyncio.sleep(0.05)
        return {"content": "...", "metadata": {}}

class SyncOcrBackend:
    def process_image(self, image_bytes: bytes, language: str) -> dict:
        # Automatically uses spawn_blocking (fallback path)
        time.sleep(0.05)
        return {"content": "...", "metadata": {}}
```

**Implementation**: `src/plugins.rs:417-510` (PythonOcrBackend::process_image)

### 2. GIL Management

#### Principle: Minimize GIL Scope

Use `Python::attach()` instead of `with_gil()` for modern PyO3:

```rust
// ✅ Good: Minimal GIL scope
let result = Python::attach(|py| {
    // GIL held only during Python code
    let coroutine = obj.call_method(...)?;
    pyo3_async_runtimes::tokio::into_future(coroutine)
})?;  // GIL released here
let value = result.await?;  // No GIL held during await

// ❌ Bad: GIL held during entire async operation
Python::with_gil(|py| async move {
    let result = some_async_op().await;  // GIL held while waiting!
    result
}).await
```

#### spawn_blocking vs block_in_place

**For long-running operations (OCR)**:
```rust
tokio::task::spawn_blocking(move || {
    Python::attach(|py| { /* OCR processing */ })
}).await?
```

**For quick operations (PostProcessor/Validator)**:
```rust
let result = tokio::task::block_in_place(|| {
    Python::attach(|py| { /* validation */ })
})?;
```

**Critical**: Using `spawn_blocking` for PostProcessor/Validator causes GIL deadlocks. Use `block_in_place` instead (see `src/plugins.rs:16-116` for detailed explanation).

### 3. Zero-Copy Data Transfer

Minimize copies across the FFI boundary:

```rust
// ✅ Good: Direct PyO3 type construction
fn to_python(py: Python, result: &ExtractionResult) -> PyResult<Bound<PyDict>> {
    let dict = PyDict::new(py);
    dict.set_item("content", PyString::new(py, &result.content))?;
    // ... construct Python objects directly
    Ok(dict)
}

// ❌ Bad: JSON round-trip
fn to_python_slow(py: Python, result: &ExtractionResult) -> PyResult<Bound<PyDict>> {
    let json = serde_json::to_string(result)?;  // Serialize to string
    let json_mod = py.import("json")?;
    json_mod.call_method1("loads", (json,))  // Parse back in Python
}
```

**Performance**: Direct construction is ~30-40% faster than JSON round-trip.

## Building

### Development Build

```bash
maturin develop --release
```

### Distribution Build

```bash
maturin build --release
```

### With Task Runner

```bash
task python:build
```

## Testing

Run Python tests that exercise the bindings:

```bash
pytest tests/
```

Benchmark async patterns:

```bash
uv run python tests/benchmark_async_simple.py
```

## Features

### Default Features

- `extension-module`: PyO3 extension module support (required for Python import)

### Optional Features

- `keywords-yake`: YAKE keyword extraction (passed through from kreuzberg)
- `keywords-rake`: RAKE keyword extraction (passed through from kreuzberg)
- `keywords`: All keyword extraction features

## Dependencies

- `pyo3 = "0.27.1"` with `abi3-py310` (stable ABI across Python 3.10+)
- `pyo3-async-runtimes = "0.27"` with `tokio-runtime` feature
- `once_cell = "1.20"` for event loop reuse
- `tokio = "1.48.0"` for async runtime
- `async-trait = "0.1.89"` for async trait methods

## Key Files

- `src/lib.rs`: Module definition, event loop initialization
- `src/core.rs`: Extraction API (extract_file, extract_bytes, batch functions)
- `src/plugins.rs`: Plugin registration (OCR, PostProcessor, Validator)
- `src/config.rs`: Configuration types
- `src/types.rs`: Result and data types
- `src/error.rs`: Exception types

## References

- **PyO3 Documentation**: https://pyo3.rs/v0.27/
- **pyo3-async-runtimes**: https://docs.rs/pyo3-async-runtimes/0.27
- **Spikard** (async patterns reference): https://github.com/Goldziher/spikard
- **Kreuzberg Core**: `../kreuzberg/`

## Performance Best Practices

### For Plugin Authors

1. **Use async for I/O-bound operations**:
   ```python
   async def process_image(self, image_bytes, language):
       async with httpx.AsyncClient() as client:
           response = await client.post(...)
       return {"content": response.json()["text"]}
   ```

2. **Use sync for CPU-bound operations**:
   ```python
   def process_image(self, image_bytes, language):
       # CPU-intensive operation
       result = ml_model.inference(image_bytes)
       return {"content": result}
   ```

3. **Call `init_async_runtime()` once at startup** (optional, auto-initializes):
   ```python
   from kreuzberg._internal_bindings import init_async_runtime
   init_async_runtime()
   ```

### For Contributors

1. **Always use `Python::attach()` not `with_gil()`** (PyO3 0.27+)
2. **Release GIL before awaiting Rust futures**
3. **Use `spawn_blocking` for long-running sync operations**
4. **Use `block_in_place` for quick sync operations (avoid GIL deadlock)**
5. **Prefer direct PyO3 type construction over JSON serialization**
6. **Check for `__await__` to detect async Python functions**

## Contributing

See the main Kreuzberg repository for contribution guidelines.
