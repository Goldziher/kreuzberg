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

### HIGH-2: Add Silent Exception Logging (2 hours)

**Priority**: P1 - Debugging Issues

**Problem**: Multiple places silently catch exceptions without logging.

**Files**:

- `kreuzberg/__init__.py:131-133, 145-146`
- `kreuzberg/postprocessors/__init__.py:65-68, 80-82, 95-97`

**Tasks**:

1. Add `logging.getLogger(__name__)` to all modules
1. Replace silent `pass` with `logger.debug()` or `logger.warning()`
1. Add `exc_info=True` for full traceback
1. Keep ImportError silent, log unexpected exceptions

______________________________________________________________________

### HIGH-3: Optimize Metadata Conversion (2 hours)

**Priority**: P1 - Performance Hot Path

**File**: `crates/kreuzberg-py/src/types.rs:81-102`

**Problem**: Recursive `serde_json::Value → PyObject` conversion on every extraction.

**Solution**: Use `pythonize` crate for efficient serialization.

**Tasks**:

1. Add `pythonize = "0.21"` to `Cargo.toml`
1. Replace manual conversion with `pythonize::pythonize()`
1. Benchmark before/after (expect 30-50% improvement)
1. Update tests

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

- **High Priority**: 7 hours (HIGH-2, HIGH-3, HIGH-4)
- **Missing Features**: 7.5 hours
- **Testing**: 6-8 hours
- **Total**: ~20.5-22.5 hours remaining

### Success Criteria

- ✅ No critical issues
- ✅ No memory leaks
- ✅ Error context preserved
- ✅ Single source of truth (no dual registries)
- 🔲 95%+ test coverage
- 🔲 Complete API exposure (chunking, cache, config)
- 🔲 Zero-copy where possible

### Recommended Next Step

#### HIGH-2: Add Silent Exception Logging

- Critical for debugging production issues
- Improves developer experience
- Minimal code changes required
- 2 hours
