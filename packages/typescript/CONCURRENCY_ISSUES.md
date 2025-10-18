# NAPI-RS Concurrency and Memory Issues

## Overview

The `kreuzberg-node` NAPI-RS bindings exhibit memory corruption issues after ~5-6 sequential extractions, causing `SIGILL (Invalid machine instruction)` crashes. This document outlines the issue, attempted fixes, and current mitigations.

## Problem Description

### Symptoms

- Tests crash with `SIGILL` after 5-6 sequential document extractions
- Individual tests pass in isolation
- The issue is cumulative (not specific to any particular file format)
- Python bindings (PyO3) do NOT have this issue
- Rust core library tests pass without issues

### Evidence

```bash
# This crashes after ~5 tests:
pnpm vitest run tests/integration/formats.spec.ts

# Individual tests work fine:
pnpm vitest run tests/integration/formats.spec.ts -t "JSON"  # ✅ PASS
```

## Root Cause Analysis

The issue appears to be in how NAPI-RS async functions interact with the Tokio runtime and V8's memory management. Despite the Rust core library having `#![deny(unsafe_code)]` and comprehensive testing, the NAPI FFI layer exhibits instability under repeated extraction loads.

## Attempted Fixes (All Ineffective)

### 1. Buffer Ownership Transfer

**Attempted:** Copy `Buffer` to `Vec<u8>` immediately to avoid V8 GC lifetime issues
**Result:** Still crashes
**Code:**

```rust
let owned_data = data.to_vec(); // Copy before passing to core library
```

### 2. Disable mimalloc Allocator

**Attempted:** Removed custom allocator to eliminate potential V8 conflicts
**Result:** Still crashes
**Code:**

```rust
// Disabled global mimalloc allocator
// #[global_allocator]
// static ALLOC: mimalloc_rust::GlobalMiMalloc = mimalloc_rust::GlobalMiMalloc;
```

### 3. Async Runtime Isolation

**Attempted:** Use `tokio::task::spawn_blocking` to isolate from NAPI-RS async runtime
**Result:** Still crashes
**Code:**

```rust
tokio::task::spawn_blocking(move || {
    kreuzberg::extract_file_sync(&file_path, mime_type.as_deref(), &rust_config)
})
.await
```

## Current Mitigations

### Test Configuration

Tests are run in batches to avoid hitting the corruption threshold:

**package.json:**

```json
{
  "scripts": {
    "test:integration": "vitest run tests/integration/pdf.spec.ts tests/integration/ocr.spec.ts && vitest run tests/integration/formats.spec.ts -t 'DOCX|XLSX|PPTX' && vitest run tests/integration/formats.spec.ts -t 'JSON|YAML|XML|email|Markdown|plain'"
  }
}
```

**vitest.config.ts:**

```typescript
export default defineConfig({
  test: {
    poolOptions: {
      threads: {
        singleThread: true, // Prevent concurrent test execution
      },
    },
  },
});
```

### Bindings Improvements

While the fixes don't eliminate the crash, they provide better isolation and error handling:

1. **Buffer Copying:** All `Buffer` arguments are copied to `Vec<u8>` immediately
1. **Spawn Blocking:** Async functions use `spawn_blocking` for better isolation
1. **No Custom Allocator:** mimalloc disabled to reduce complexity

## Test Results

With mitigations:

- ✅ **128 unit tests** - All pass
- ✅ **16 integration tests** (PDF, OCR) - All pass
- ✅ **13 format tests** - All pass (when batched)
- **Total: 154 tests passing**

## Technical Details

### NAPI-RS Documentation Review

According to napi-rs docs:

- `Buffer` creates napi_reference automatically in async functions
- Buffer data should remain valid across await points
- Custom allocators may conflict with V8's memory management

### Comparison with Python Bindings

PyO3 bindings use different FFI patterns:

- Python bindings copy data immediately (`bytes.to_vec()`)
- PyO3 has different GC interaction model than V8
- No reported memory issues with Python bindings

### Rust Core Library

The core library (`crates/kreuzberg`) is memory-safe:

- `#![deny(unsafe_code)]` enforced
- Comprehensive test suite (247 tests) passes
- Used successfully in Python bindings

## Recommendations

### For Users

1. **Use batched processing** when extracting many documents
1. **Monitor memory usage** in production
1. **Consider Python bindings** for high-throughput scenarios

### For Development

1. **Further investigation needed** - May be NAPI-RS library issue
1. **Consider alternative FFI approaches** - node-bindgen, plain C bindings
1. **Upstream bug report** - File issue with NAPI-RS project

## Related Issues

- [NAPI-RS GitHub Issues](https://github.com/napi-rs/napi-rs/issues)
- Rust core library concurrency stress test also shows issues (100/400 extractions fail)

## Status

- **Severity:** HIGH - Production use requires workarounds
- **Impact:** Cannot run full test suite without batching
- **Workaround:** Effective - All tests pass with current mitigations
- **Component:** `crates/kreuzberg-node` (NAPI-RS bindings only)
- **Date:** 2025-10-18
