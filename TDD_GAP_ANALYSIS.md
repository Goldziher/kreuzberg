# TDD Gap Analysis: Plugin API Parity

**Generated**: 2025-11-20
**Branch**: `feature/close-plugin-api-gaps`
**Approach**: Test-Driven Development (Red-Green-Refactor)

## Overview

Created comprehensive E2E test suites for plugin APIs across all language bindings. Tests were written first to identify missing functionality, following TDD methodology.

## Test Files Created

- ✅ `e2e/python/tests/test_plugin_apis.py` - 15 tests
- ✅ `e2e/typescript/tests/plugin-apis.test.ts` - 15 tests
- ✅ `e2e/ruby/spec/plugin_apis_spec.rb` - 15 tests
- ✅ `e2e/java/src/test/java/dev/kreuzberg/e2e/PluginAPIsTest.java` - 15 tests
- ✅ `e2e/go/plugin_apis_test.go` - 15 tests

## Python Binding - Test Results

**Result**: 8 failures, 7 passed (53% passing)

### ✅ Passing Tests (Already Implemented)
1. `test_list_validators` - ✅ PASS
2. `test_clear_validators` - ✅ PASS
3. `test_list_post_processors` - ✅ PASS
4. `test_clear_post_processors` - ✅ PASS
5. `test_list_ocr_backends` - ✅ PASS
6. `test_unregister_ocr_backend` - ✅ PASS
7. `test_config_from_file` - ✅ PASS

### ❌ Failing Tests (Missing APIs)

#### 1. OCR Backend Management
- ❌ `clear_ocr_backends()` - Missing function

#### 2. Document Extractor Management (All Missing - P0)
- ❌ `list_document_extractors()` - Missing function
- ❌ `unregister_document_extractor()` - Missing function
- ❌ `clear_document_extractors()` - Missing function

#### 3. Configuration APIs
- ❌ `ExtractionConfig.discover()` - Missing class method

#### 4. MIME Utilities (All Missing - P0)
- ❌ `detect_mime_type(bytes)` - Missing function
- ❌ `detect_mime_type_from_path(path)` - Missing function
- ❌ `get_extensions_for_mime(mime_type)` - Missing function

## Implementation Priority

### P0 (Critical - Core Extensibility)
1. **Document Extractor APIs** (3 functions)
   - `list_document_extractors()`
   - `unregister_document_extractor(name)`
   - `clear_document_extractors()`

2. **MIME Utilities** (3 functions)
   - `detect_mime_type(content: bytes) -> str`
   - `detect_mime_type_from_path(path: str) -> str`
   - `get_extensions_for_mime(mime_type: str) -> List[str]`

### P1 (Important - Developer Experience)
3. **Configuration Discovery**
   - `ExtractionConfig.discover() -> Optional[ExtractionConfig]`

4. **OCR Backend Cleanup**
   - `clear_ocr_backends()`

## Implementation Status

### ✅ Phase 1: Rust Core APIs - COMPLETED

**Existing APIs (already implemented):**
- ✅ Document extractor management (`list_extractors`, `unregister_extractor`, `clear_extractors`)
- ✅ OCR backend clearing (`clear_ocr_backends`)
- ✅ Config loading (`ExtractionConfig::from_file`, `ExtractionConfig::discover`)

**New APIs Added:**
- ✅ `detect_mime_type_from_bytes(content: &[u8]) -> Result<String>` - NEW
- ✅ `get_extensions_for_mime(mime_type: &str) -> Result<Vec<String>>` - NEW
- ✅ Added `infer` crate dependency for magic byte detection
- ✅ Exposed in public API via `crates/kreuzberg/src/lib.rs`

**Test Status:**
- ✅ Rust core compiles successfully
- ✅ All 8 missing Python APIs now have Rust implementations

### Phase 2: Implement in Language Bindings (Red → Green)
For each failing test:
1. Implement in `crates/kreuzberg-py/src/lib.rs`
2. Expose in `packages/python/kreuzberg/__init__.py`
3. Run test → should pass
4. Repeat for next failing test

### Phase 3: Replicate Across All Bindings
Apply same implementations to:
- TypeScript (NAPI-RS)
- Ruby (Magnus)
- Java (FFM API)
- Go (CGo)

### Phase 4: Verification
- [ ] Run all E2E tests
- [ ] Verify 100% test pass rate
- [ ] Commit and push

## Test Coverage by Category

| Category | Tests | Passing | Missing |
|----------|-------|---------|---------|
| Validators | 2 | 2 (100%) | 0 |
| Post-processors | 2 | 2 (100%) | 0 |
| OCR Backends | 3 | 2 (67%) | 1 |
| **Document Extractors** | **3** | **0 (0%)** | **3** |
| **Configuration** | **2** | **1 (50%)** | **1** |
| **MIME Utilities** | **3** | **0 (0%)** | **3** |
| **TOTAL** | **15** | **7 (47%)** | **8** |

## Expected Final State

After implementation, all bindings should have:
- ✅ 15/15 tests passing (100%)
- ✅ Full plugin API parity with Rust core
- ✅ Complete MIME detection utilities
- ✅ Configuration discovery support
- ✅ Document extractor registration support

## Notes

- Tests follow language-specific conventions (pytest for Python, RSpec for Ruby, etc.)
- Each test is self-contained with proper setup/teardown
- Tests use temporary directories for file operations
- Error handling tests ensure graceful failure for nonexistent items
