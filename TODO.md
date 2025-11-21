# API Parity Gaps - TDD Implementation Plan

**Generated**: 2025-11-21
**Branch**: `feature/close-all-api-gaps`
**Approach**: Test-Driven Development (Red-Green-Refactor)

---

## Overview

Following comprehensive API parity review, identified critical gaps across language bindings. This document tracks TDD-based implementation to achieve 100% API parity.

**Strategy**:
1. Write failing E2E tests for ALL missing APIs
2. Implement Rust core APIs (if needed)
3. Implement binding-specific wrappers
4. Verify tests pass
5. Commit and push

---

## Priority Breakdown

### P0: Critical Functionality Gaps (MUST FIX)

#### 1. Missing `register_document_extractor()` API
**Status**: ❌ Missing in ALL bindings
**Severity**: Critical - Users cannot register custom document extractors

**Affected**:
- Python: `packages/python/kreuzberg/__init__.py`
- TypeScript: `packages/typescript/src/index.ts`
- Ruby: `packages/ruby/lib/kreuzberg.rb`
- Java: `packages/java/src/main/java/dev/kreuzberg/Kreuzberg.java`
- Go: `packages/go/kreuzberg/plugins.go`

**Rust Core**: ✅ Already exports `register_extractor()` at `crates/kreuzberg/src/plugins/mod.rs:203`

**E2E Tests to Add**:
- [ ] `e2e/python/tests/test_plugin_apis.py` - `test_register_document_extractor`
- [ ] `e2e/typescript/tests/plugin-apis.test.ts` - `should register a document extractor`
- [ ] `e2e/ruby/spec/plugin_apis_spec.rb` - `register_document_extractor`
- [ ] `e2e/java/src/test/java/dev/kreuzberg/e2e/PluginAPIsTest.java` - `testRegisterDocumentExtractor`
- [ ] `e2e/go/plugin_apis_test.go` - `TestRegisterDocumentExtractor`

**Implementation Steps**:
1. Add E2E tests (should fail - RED)
2. Python: Add binding in `crates/kreuzberg-py/src/plugins.rs`, export in `__init__.py`
3. TypeScript: Add binding in `crates/kreuzberg-node/src/lib.rs`, export in `index.ts`
4. Ruby: Add binding in `packages/ruby/ext/kreuzberg_rb/native/src/lib.rs`, export in `kreuzberg.rb`
5. Java: Add FFI function in `crates/kreuzberg-ffi/src/lib.rs`, wrapper in `Kreuzberg.java`
6. Go: Add CGo function in `packages/go/kreuzberg/plugins.go`
7. Run tests (should pass - GREEN)

---

### P1: High Priority - Architectural Inconsistencies

#### 2. Rust Core Missing Post-Processor Mutation APIs
**Status**: ⚠️ Rust core only exports `list_post_processors()`, not mutation APIs
**Severity**: High - Architectural inconsistency

**Issue**:
- Rust core does NOT export: `register_post_processor`, `unregister_post_processor`, `clear_post_processors`
- All bindings work around this by accessing `get_post_processor_registry()` directly
- Inconsistent with validators and OCR backends which DO export mutation APIs

**Files**:
- `crates/kreuzberg/src/plugins/processor.rs` - Functions exist but not re-exported
- `crates/kreuzberg/src/plugins/mod.rs:207` - Only exports `list_post_processors()`

**E2E Tests**: ✅ Already exist (bindings work around the gap)

**Implementation Steps**:
1. Export `register_post_processor`, `unregister_post_processor`, `clear_post_processors` in `plugins/mod.rs`
2. Update bindings to use exported functions instead of direct registry access (optional cleanup)

---

#### 3. Python Missing Config Loading APIs
**Status**: ❌ Python missing `ExtractionConfig.from_file()` and `ExtractionConfig.discover()`
**Severity**: High - Missing developer convenience

**Current State**:
- TypeScript ✅ Has: `ExtractionConfig.fromFile()`, `ExtractionConfig.discover()`
- Java ✅ Has: `ExtractionConfig.fromFile()`, `ExtractionConfig.discover()`
- Go ✅ Has: `ConfigFromFile()`, `ConfigDiscover()`
- Python ❌ Missing both
- Ruby ❌ Missing both

**Rust Core**: ✅ Exports `ExtractionConfig::from_file()` and `ExtractionConfig::discover()`

**E2E Tests to Add**:
- [ ] `e2e/python/tests/test_plugin_apis.py` - Already exists but not using class methods
- [ ] `e2e/ruby/spec/plugin_apis_spec.rb` - Already exists but not using class methods

**Implementation Steps**:
1. **Python**: Add class methods `from_file()` and `discover()` to `ExtractionConfig` in `crates/kreuzberg-py/src/config.rs`
2. **Ruby**: Add class methods `from_file` and `discover` to `Config::Extraction` in `packages/ruby/ext/kreuzberg_rb/native/src/lib.rs`
3. Update E2E tests to use class methods instead of standalone functions
4. Verify tests pass

---

#### 4. Ruby Missing Config Loading APIs
**Status**: ❌ Same as Python above
**See**: Section 3 above for implementation

---

### P2: Medium Priority - Feature Completeness

#### 5. Ruby Missing MIME Utilities (All 4 APIs)
**Status**: ❌ Ruby missing ALL MIME utility functions
**Severity**: Medium - Missing utility features

**Missing APIs**:
- `detect_mime_type(data)` / `detect_mime_type_from_bytes(data)`
- `detect_mime_type_from_path(path)`
- `get_extensions_for_mime(mime_type)`
- `validate_mime_type(mime_type)`

**Current State**:
- Python ✅ Has 3/4 (missing `validate_mime_type`)
- TypeScript ✅ Has all 4
- Java ✅ Has all 4
- Go ✅ Has all 4
- Ruby ❌ Has 0/4

**E2E Tests**: ✅ Already exist in `e2e/ruby/spec/plugin_apis_spec.rb` but commented out or skipped

**Implementation Steps**:
1. Add bindings in `packages/ruby/ext/kreuzberg_rb/native/src/lib.rs` for all 4 MIME functions
2. Export in `packages/ruby/lib/kreuzberg.rb`
3. Uncomment/enable E2E tests
4. Verify tests pass

---

#### 6. Ruby Missing Embedding Preset APIs
**Status**: ❌ Ruby missing `list_embedding_presets()` and `get_embedding_preset()`
**Severity**: Medium - Feature gap

**Current State**:
- Python ✅ Has both
- TypeScript ✅ Has both
- Java ✅ Has both
- Go ✅ Has both
- Ruby ❌ Has neither

**E2E Tests to Add**:
- [ ] `e2e/ruby/spec/plugin_apis_spec.rb` - `list_embedding_presets`, `get_embedding_preset`

**Implementation Steps**:
1. Add bindings in `packages/ruby/ext/kreuzberg_rb/native/src/lib.rs`
2. Export in `packages/ruby/lib/kreuzberg.rb`
3. Add E2E tests
4. Verify tests pass

---

#### 7. Ruby Missing `unregister_document_extractor()`
**Status**: ❌ Ruby has `list_document_extractors` and `clear_document_extractors` but NOT `unregister_document_extractor`
**Severity**: Medium - Incomplete API set

**Current State**:
- Python ✅ Has
- TypeScript ✅ Has
- Java ✅ Has
- Go ✅ Has
- Ruby ❌ Missing (but has list and clear)

**E2E Tests**: ✅ Already exists in `e2e/ruby/spec/plugin_apis_spec.rb`

**Implementation Steps**:
1. The native binding likely already exists in `packages/ruby/ext/kreuzberg_rb/native/src/lib.rs`
2. Just needs to be added to `module_function` list in `packages/ruby/lib/kreuzberg.rb`
3. Verify E2E test passes

---

### P3: Low Priority - Nice to Have

#### 8. Python Missing `validate_mime_type()`
**Status**: ❌ Python missing 1 MIME utility
**Severity**: Low - Minor utility function

**Current State**:
- Python has: `detect_mime_type()`, `detect_mime_type_from_path()`, `get_extensions_for_mime()`
- Python missing: `validate_mime_type()`

**E2E Tests to Add**:
- [ ] `e2e/python/tests/test_plugin_apis.py` - `test_validate_mime_type`

**Implementation Steps**:
1. Add binding in `crates/kreuzberg-py/src/lib.rs`
2. Export in `packages/python/kreuzberg/__init__.py`
3. Add E2E test
4. Verify test passes

---

## TDD Implementation Checklist

### Phase 1: Write Failing Tests (RED)
- [ ] Add E2E test for `register_document_extractor()` in Python
- [ ] Add E2E test for `register_document_extractor()` in TypeScript
- [ ] Add E2E test for `register_document_extractor()` in Ruby
- [ ] Add E2E test for `register_document_extractor()` in Java
- [ ] Add E2E test for `register_document_extractor()` in Go
- [ ] Add E2E test for `ExtractionConfig.from_file()` in Python (class method)
- [ ] Add E2E test for `ExtractionConfig.discover()` in Python (class method)
- [ ] Add E2E test for `Config::Extraction.from_file` in Ruby (class method)
- [ ] Add E2E test for `Config::Extraction.discover` in Ruby (class method)
- [ ] Add E2E tests for Ruby MIME utilities (4 tests)
- [ ] Add E2E tests for Ruby embedding presets (2 tests)
- [ ] Add E2E test for Python `validate_mime_type()`
- [ ] Run all E2E tests → verify failures

### Phase 2: Implement Rust Core (if needed)
- [ ] Export post-processor mutation APIs in `plugins/mod.rs` (P1)
- [ ] Verify Rust core has all required APIs for bindings

### Phase 3: Implement Bindings (GREEN)
**P0 - Critical**:
- [ ] Python: `register_document_extractor()`
- [ ] TypeScript: `registerDocumentExtractor()`
- [ ] Ruby: `register_document_extractor`
- [ ] Java: `registerDocumentExtractor()`
- [ ] Go: `RegisterDocumentExtractor()`

**P1 - High**:
- [ ] Python: `ExtractionConfig.from_file()`, `ExtractionConfig.discover()`
- [ ] Ruby: `Config::Extraction.from_file`, `Config::Extraction.discover`

**P2 - Medium**:
- [ ] Ruby: `detect_mime_type()`, `detect_mime_type_from_path()`, `get_extensions_for_mime()`, `validate_mime_type()`
- [ ] Ruby: `list_embedding_presets()`, `get_embedding_preset()`
- [ ] Ruby: `unregister_document_extractor()` (expose existing)

**P3 - Low**:
- [ ] Python: `validate_mime_type()`

### Phase 4: Verification
- [ ] Run Python E2E tests → 100% passing
- [ ] Run TypeScript E2E tests → 100% passing
- [ ] Run Ruby E2E tests → 100% passing
- [ ] Run Java E2E tests → 100% passing
- [ ] Run Go E2E tests → 100% passing
- [ ] Run `cargo clippy --all-targets --all-features` → zero warnings
- [ ] Commit and push

### Phase 5: Cleanup
- [ ] Remove TODO.md
- [ ] Create PR with all changes
- [ ] Document remaining architectural decisions (if any)

---

## Expected Final State

After completion, ALL bindings should have:

| API Category | Python | TypeScript | Ruby | Java | Go |
|--------------|--------|------------|------|------|----|
| **Validators** (4 APIs) | ✅ 4/4 | ✅ 4/4 | ✅ 4/4 | ✅ 4/4 | ✅ 4/4 |
| **Post-Processors** (4 APIs) | ✅ 4/4 | ✅ 4/4 | ✅ 4/4 | ✅ 4/4 | ✅ 4/4 |
| **OCR Backends** (4 APIs) | ✅ 4/4 | ✅ 4/4 | ✅ 4/4 | ✅ 4/4 | ✅ 4/4 |
| **Document Extractors** (4 APIs) | ⏳ 3/4 | ⏳ 3/4 | ⏳ 2/4 | ⏳ 3/4 | ⏳ 3/4 |
| **Config Loading** (2 APIs) | ⏳ 0/2 | ✅ 2/2 | ⏳ 0/2 | ✅ 2/2 | ✅ 2/2 |
| **MIME Utilities** (4 APIs) | ⏳ 3/4 | ✅ 4/4 | ⏳ 0/4 | ✅ 4/4 | ✅ 4/4 |
| **Embedding Presets** (2 APIs) | ✅ 2/2 | ✅ 2/2 | ⏳ 0/2 | ✅ 2/2 | ✅ 2/2 |

**Legend**: ✅ Complete | ⏳ Work needed

**Target**: All cells should be ✅ (100% parity)

---

## Notes

- Follow TDD strictly: Write test → See it fail → Implement → See it pass
- Each implementation should be committed separately by priority
- Ruby has the most gaps (13 missing APIs)
- Python has 3 missing APIs
- TypeScript/Java/Go are nearly complete (1 missing each)
- Use specialized agents for parallel implementation where possible

**End of TODO.md**
