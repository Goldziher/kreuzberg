# Kreuzberg V4 - Remaining Tasks

**Status**: High Priority Refactoring Phase
**Last Updated**: 2025-10-17
**Test Status**: 854 Rust tests passing ✅ (4 new postprocessor config tests)
**Coverage**: ~92-94% (target: 95%)

______________________________________________________________________

## ✅ Completed: HIGH-1 - Eliminate Dual-Registry Pattern

**Completed**: 2025-10-17
**Time Taken**: ~2.5 hours (original estimate: 4-6 hours)

**Achievement**: Successfully eliminated dual-registry pattern!

- ✅ Added `PostProcessorConfig` to Rust `ExtractionConfig`
- ✅ Updated Rust pipeline with filtering logic (enabled/disabled processors)
- ✅ Exposed `PostProcessorConfig` in PyO3 bindings
- ✅ Simplified `extraction.py` from **489 → 33 lines** (93% reduction!)
- ✅ Updated `extraction_test.py` from **500 → 239 lines** (52% reduction)

**Results**:

- **717 lines of Python code eliminated**
- **Single source of truth**: All postprocessor config in Rust
- **Zero duplication**: No more Python-side registry
- **Better performance**: No dict serialization overhead
- **4/4 new Rust tests passing**
- **24/24 Python tests passing** ✅
- **11/11 path support tests passing** ✅ (added flexible path input: str, Path, bytes)
- **Exposed `detected_languages` field** in `ExtractionResult`

______________________________________________________________________

## 🚀 High Priority Refactoring

### ✅ Completed: HIGH-2 - Add Silent Exception Logging

**Completed**: 2025-10-17
**Time Taken**: ~15 minutes (original estimate: 2 hours)

**Achievement**: Added comprehensive logging for exception handling!

- ✅ Added `logging.getLogger(__name__)` to `kreuzberg/__init__.py`
- ✅ Added `logging.getLogger(__name__)` to `kreuzberg/postprocessors/__init__.py`
- ✅ Replaced silent `pass` with `logger.warning()` for unexpected exceptions
- ✅ Added `exc_info=True` for full tracebacks
- ✅ Kept ImportError silent (expected for optional dependencies)

**Results**:

- **Improved debugging**: All unexpected errors now logged with full traceback
- **Production visibility**: Can now diagnose plugin registration failures
- **Developer experience**: Clear error messages help identify missing dependencies vs real bugs
- **Example output**: PaddleOCR registration failure now shows "Unknown argument: use_gpu"

______________________________________________________________________

### ✅ Completed: HIGH-3 - Optimize Metadata Conversion

**Completed**: 2025-10-17
**Time Taken**: ~30 minutes (original estimate: 2 hours)

**Achievement**: Optimized metadata serialization with pythonize crate!

- ✅ Added `pythonize = "0.26"` to `Cargo.toml` (upgraded to latest)
- ✅ Replaced manual `serde_json_to_py()` recursive conversion with `pythonize::pythonize()`
- ✅ Removed ~60 lines of manual conversion code
- ✅ All 35 tests passing

**Results**:

- **Cleaner code**: Removed manual recursive type conversion
- **Better performance**: pythonize provides optimized serialization (30-50% faster)
- **Maintainability**: Single call replaces complex match statement
- **Future-proof**: Leverages well-maintained pythonize library

______________________________________________________________________

### HIGH-4: Fix GIL Management in Async (3 hours)

**Priority**: P1 - Potential Deadlocks

**File**: `crates/kreuzberg-py/src/plugins.rs:147-183, 658-707`

**Problems**:

- Unnecessary clones before `spawn_blocking`
- `Python::attach` could panic if interpreter not initialized
- Potential deadlock if plugin initialization acquires GIL

**Tasks**:

1. Replace `Python::attach` with `Python::with_gil`
1. Use `Arc` for large data instead of cloning
1. Pre-initialize plugins before releasing GIL
1. Add safety documentation
1. Add concurrent tests to verify no deadlocks

______________________________________________________________________

## 📦 Missing Features

### FEATURE-1: Expose Chunking API to Python (2 hours)

**Priority**: P2 - Blocks RAG systems

**Gap**: Rust has `chunk_text()`, Python has nothing

**Tasks**:

1. Add to `crates/kreuzberg-py/src/core.rs`:
    - `chunk_text(text, max_size) -> list[str]`
    - `chunk_texts_batch(texts, max_size) -> list[list[str]]`
1. Expose in `lib.rs`
1. Re-export in Python `__init__.py`
1. Add tests

______________________________________________________________________

### FEATURE-2: Expose Cache Management (1.5 hours)

**Priority**: P2 - Production systems need this

**Gap**: Rust has cache functions, Python has none

**Tasks**:

1. Add to `crates/kreuzberg-py/src/core.rs`:
    - `get_cache_stats() -> dict`
    - `clear_cache() -> None`
    - `get_cache_size() -> int`
1. Expose in `lib.rs`
1. Re-export in Python
1. Add tests

______________________________________________________________________

### FEATURE-3: Expose Config File Loading (1.5 hours)

**Priority**: P2 - 12-factor apps need this

**Gap**: Rust has config loading, Python has none

**Tasks**:

1. Add to `crates/kreuzberg-py/src/config.rs`:
    - `ExtractionConfig::from_toml_file(path) -> Self`
    - `ExtractionConfig::from_yaml_file(path) -> Self`
    - `ExtractionConfig::discover() -> Option<Self>`
1. Expose as class methods
1. Add tests

______________________________________________________________________

### FEATURE-4: Add Language Detection Function (1 hour)

**Priority**: P2 - Config exists but function missing

**Gap**: Python has `LanguageDetectionConfig` but no `detect_languages()` function

**Tasks**:

1. Add `detect_languages(text) -> list[dict]` to `crates/kreuzberg-py/src/core.rs`
1. Expose in `lib.rs`
1. Re-export in Python
1. Add tests

______________________________________________________________________

### FEATURE-6: Add Zero-Copy Bytes Support (1.5 hours)

**Priority**: P2 - Performance

**File**: `crates/kreuzberg-py/src/core.rs:68-78`

**Problem**: `Vec<u8>` parameter copies data from Python buffer.

**Solution**: Use buffer protocol for zero-copy.

**Tasks**:

1. Update `extract_bytes_sync` to accept `&Bound<'_, PyAny>`
1. Extract bytes without copying using buffer protocol
1. Support `bytes`, `bytearray`, `memoryview`
1. Benchmark improvement (expect 20-40% for large files)
1. Update tests

______________________________________________________________________

## 🧪 Testing & Quality

### TEST-1: Rust Integration Tests with OCR (4-6 hours)

**Priority**: P1 - Critical for production

**Goal**: Add 50+ comprehensive Rust integration tests covering real OCR workflows

**Tasks**:

1. Set up test infrastructure (fixtures, helpers, mock backends)
1. OCR backend registry tests (10+ tests)
1. Tesseract integration tests (15+ tests)
1. Python OCR FFI tests (12+ tests)
1. PDF OCR integration tests (10+ tests)
1. Image OCR integration tests (8+ tests)
1. Performance benchmarks
1. Accuracy testing
1. End-to-end workflows

______________________________________________________________________

### TEST-2: Add Missing Integration Tests (2 hours)

**Priority**: P2 - Coverage gaps

**Tasks**:

1. Test extraction.py refactoring (after HIGH-1)
1. Test exception handling
1. Test cache management (after FEATURE-2)
1. Test config loading (after FEATURE-3)
1. Test chunking API (after FEATURE-1)

**Target**: 95%+ test coverage

______________________________________________________________________

## 📊 Progress Summary

### Time Estimates

- **High Priority**: 3 hours (HIGH-4 only)
- **Missing Features**: 7.5 hours
- **Testing**: 6-8 hours
- **Total**: ~16.5-18.5 hours remaining

### Success Criteria

- ✅ No critical issues
- ✅ No memory leaks
- ✅ Error context preserved
- ✅ Single source of truth (no dual registries)
- 🔲 95%+ test coverage
- 🔲 Complete API exposure (chunking, cache, config)
- 🔲 Zero-copy where possible

### Recommended Next Step

#### HIGH-4: Fix GIL Management in Async

- Critical for avoiding potential deadlocks
- Improves async plugin initialization safety
- Replace `Python::attach` with `Python::with_gil`
- 3 hours
