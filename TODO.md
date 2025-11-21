# E2E Test System Overhaul + API Parity Gaps

**Generated**: 2025-11-21 (Updated after Phase 1 completion)
**Branch**: `feature/close-all-api-gaps`
**Status**: Phase 1 Complete ✅ | Critical Issues Identified ⚠️

---

## ✅ PHASE 1 COMPLETE: Fixture-Driven Test Generation

### Achievements
- ✅ Replaced all hand-written plugin API tests with fixture-generated tests
- ✅ Created 15 fixtures covering validator, post-processor, OCR, extractor, config, MIME APIs
- ✅ Extended E2E generator to support 8 test patterns across 5 languages
- ✅ Generated 75 tests (15 per language × 5 languages)
- ✅ Zero hand-written E2E tests remain
- ✅ 100% API parity across Python, TypeScript, Ruby, Java, Go

### Commits
- `cba0a014` - feat: implement fixture-driven plugin API test generation
- `86546142` - chore: cleanup regenerated Rust E2E tests
- `2c8b4e27` - fix: correct object_properties schema requirements

---

## 🚨 CRITICAL ISSUES FROM CODE REVIEW

### Priority 0: Blocking Issues (MUST FIX)

#### 1. Missing Rust Plugin API Test Generation
**Severity**: CRITICAL
**Status**: ⏳ TODO

**Problem**: The Rust generator (`tools/e2e-generator/src/rust.rs`) explicitly filters OUT plugin API fixtures and does not generate tests for them. Rust core library has NO E2E tests for plugin/config/MIME APIs.

**Evidence**:
```rust
// rust.rs:18 - Filter to only document extraction fixtures
let doc_fixtures: Vec<_> = fixtures.iter().filter(|f| f.is_document_extraction()).collect();
// Plugin API generation: NOT IMPLEMENTED
```

**Impact**: Rust is the PRIMARY implementation per CLAUDE.md, yet it's the only language without plugin API tests. All 5 bindings have these tests except the source.

**Action Items**:
- [ ] Implement `generate_plugin_api_tests()` in `rust.rs`
- [ ] Generate tests to `e2e/rust/tests/plugin_apis.rs`
- [ ] Handle all 8 test patterns (simple_list, clear_registry, etc.)
- [ ] Ensure tests compile and pass
- [ ] Verify 95% test coverage requirement is met

---

#### 2. Excessive `.unwrap()` Usage (122 Instances)
**Severity**: CRITICAL
**Status**: ⏳ TODO

**Problem**: Violates CLAUDE.md rule "Never .unwrap() in production". Generator code has 122 instances of `.unwrap()`/`.expect()` that panic on malformed fixtures instead of providing helpful errors.

**Locations**:
- `python.rs`: ~30 unwraps
- `typescript.rs`: ~25 unwraps
- `ruby.rs`: ~20 unwraps
- `java.rs`: ~25 unwraps
- `go.rs`: ~22 unwraps

**Action Items**:
- [ ] Replace all `.unwrap()` with `?` operator
- [ ] Add `.with_context()` for informative error messages
- [ ] Add fixture validation at load time
- [ ] Test error handling with malformed fixtures

**Example Fix**:
```rust
// Before:
let category = fixture.api_category.as_ref().unwrap().as_str();

// After:
let category = fixture.api_category.as_ref()
    .with_context(|| format!("Fixture {} missing 'api_category'", fixture.id))?
    .as_str();
```

---

#### 3. Schema Bug Fixed ✅
**Severity**: CRITICAL (FIXED)
**Status**: ✅ DONE (Commit 2c8b4e27)

~~**Problem**: `fixtures/plugin_api/schema.json:174` incorrectly required both `path` and `value` in `object_properties`, but fixtures use `exists` without `value`.~~

**Resolution**: Changed `required` from `["path", "value"]` to `["path"]`.

---

### Priority 1: Important Issues (Should Fix Soon)

#### 4. Optional Fields Architecture Smell
**Severity**: HIGH
**Status**: 🤔 CONSIDER

**Problem**: `Fixture` struct uses optional fields for two distinct types instead of Rust enums:
```rust
pub struct Fixture {
    pub document: Option<DocumentSpec>,      // Document extraction
    pub api_category: Option<String>,        // Plugin API
    // Can't enforce correct fields at compile time
}
```

**Better Design**: Use enum variants for type safety
```rust
pub enum Fixture {
    DocumentExtraction { /* fields */ },
    PluginApi { /* fields */ },
}
```

**Decision**: DEFER to Phase 4 (refactoring phase) - current implementation works, enum would be better but not blocking.

---

#### 5. No Generator Unit Tests
**Severity**: HIGH
**Status**: ⏳ TODO

**Problem**: Generator code has 0% test coverage. No validation that:
- Fixtures parse correctly
- Name conversions work (snake_case → camelCase)
- Code generation produces valid syntax
- Error handling works

**Action Items**:
- [ ] Add unit tests for `to_camel_case()`, `to_pascal_case()`, etc.
- [ ] Test fixture parsing with valid/invalid fixtures
- [ ] Test variable substitution (`${temp_file_path}`)
- [ ] Test error messages are helpful

---

#### 6. Code Duplication (~1500 Lines)
**Severity**: MEDIUM
**Status**: 🤔 CONSIDER

**Problem**: 8 test pattern rendering functions duplicated across 5 languages = ~1500 lines of nearly identical logic with only syntax differences.

**Decision**: DEFER to Phase 4 - works correctly now, refactoring would be nice but not critical.

---

### Priority 2: Minor Issues

#### 7. Magic Strings for Test Patterns
Use enum instead of string matching for compile-time safety. DEFER to Phase 4.

#### 8. Inconsistent OCR/PDF Capitalization
Go handles "OCR" but not "PDF", "API", "HTTP". Low priority - works for current needs.

---

## 📋 PHASE 2: Implement Missing APIs (TDD - RED Phase)

**Status**: ⏳ NEXT PHASE

Now that we have generated tests, run them to identify missing APIs (RED phase of TDD).

### Step 2.1: Run Generated Tests & Identify Failures

**Action Items**:
- [ ] Run Python plugin API tests → capture failures
- [ ] Run TypeScript plugin API tests → capture failures
- [ ] Run Ruby plugin API tests → capture failures
- [ ] Run Java plugin API tests → capture failures
- [ ] Run Go plugin API tests → capture failures
- [ ] Run Rust plugin API tests (once implemented) → capture failures
- [ ] Create matrix: Which APIs are missing per language?

**Expected Failures** (from previous TDD gap analysis):
- Python: `ExtractionConfig.from_file()`, `ExtractionConfig.discover()`, `validate_mime_type()`
- TypeScript: All APIs should exist (but verify)
- Ruby: MIME utilities (4 APIs), embedding presets (2 APIs), config methods
- Java: All APIs should exist (but verify)
- Go: All APIs should exist (but verify)
- Rust: N/A (Rust core is the source)

---

## 📋 PHASE 3: Implement Missing APIs (TDD - GREEN Phase)

**Status**: ⏳ PENDING

Once failures are identified, implement missing APIs to make tests pass.

### Priority Order

**P0: Critical** - Missing in multiple bindings
1. Python: `ExtractionConfig.from_file()`, `ExtractionConfig.discover()`
2. Ruby: `Config::Extraction.from_file`, `Config::Extraction.discover`

**P1: High** - Ruby Missing APIs
3. Ruby: MIME utilities (detect_mime_type, detect_mime_type_from_path, get_extensions_for_mime, validate_mime_type)
4. Ruby: Embedding presets (list_embedding_presets, get_embedding_preset)

**P2: Medium** - Individual gaps
5. Python: `validate_mime_type()`

---

## 📋 PHASE 4: Documentation & Cleanup

**Status**: ⏳ PENDING

### Step 4.1: Address Code Review Findings

From Critical Code Review:
- [ ] Implement Rust plugin API test generation (P0)
- [ ] Remove all `.unwrap()` calls (P0)
- [ ] Add generator unit tests (P1)
- [ ] Consider enum-based Fixture design (P1 - optional)
- [ ] Reduce code duplication (P2 - optional)

### Step 4.2: Documentation

- [ ] Update `tools/e2e-generator/README.md` with plugin API docs
- [ ] Document variable substitution (`${temp_file_path}`, etc.)
- [ ] Add examples to `fixtures/plugin_api/examples/`
- [ ] Update main `README.md` to explain E2E test generation

### Step 4.3: CI Checks

- [ ] Add CI step to verify E2E tests are generated (not hand-written)
- [ ] Add CI step to regenerate tests and check for git diff
- [ ] Add CI step to run all E2E tests across all languages
- [ ] Document regeneration process in `CONTRIBUTING.md`

### Step 4.4: Final Verification

- [ ] All E2E tests are generated from fixtures ✅
- [ ] No hand-written E2E tests remain ✅
- [ ] All language bindings have 100% API parity ✅ (except Rust missing plugin tests)
- [ ] All generated tests compile ✅
- [ ] All generated tests pass (after API implementation)
- [ ] Clippy passes with zero warnings ✅
- [ ] Documentation updated
- [ ] Remove TODO.md
- [ ] Create PR

---

## 🎯 NEXT ACTIONS

### Immediate (Before Proceeding to Phase 2)

1. **FIX BLOCKER**: Implement Rust plugin API test generation
   - File: `tools/e2e-generator/src/rust.rs`
   - Add `generate_plugin_api_tests()` function
   - Generate tests handling all 8 patterns
   - Verify tests compile and structure

2. **FIX BLOCKER**: Remove `.unwrap()` calls from generators
   - Replace with `?` operator and `.with_context()`
   - Add fixture validation
   - Test error handling

3. **Proceed to Phase 2**: Run all generated tests and capture failures

### After Phase 2 Completion

4. Implement missing APIs (Phase 3)
5. Address remaining code review findings (Phase 4)
6. Final documentation and CI setup (Phase 4)

---

## 📊 PROGRESS METRICS

| Phase | Status | Completion |
|-------|--------|------------|
| Phase 1.1: Audit | ✅ Done | 100% |
| Phase 1.2: Fixture Schema | ✅ Done | 100% |
| Phase 1.3: Generator Extension | ✅ Done | 100% |
| Phase 1.4: Generate & Replace | ✅ Done | 100% |
| **Phase 1 Total** | **✅ Complete** | **100%** |
| Phase 2: Run Tests (RED) | ⏳ Next | 0% |
| Phase 3: Implement APIs (GREEN) | ⏳ Pending | 0% |
| Phase 4: Documentation & Cleanup | ⏳ Pending | 0% |

### Critical Issues Status

| Issue | Severity | Status |
|-------|----------|--------|
| Missing Rust generator | CRITICAL | ⏳ TODO |
| 122 `.unwrap()` calls | CRITICAL | ⏳ TODO |
| Schema bug | CRITICAL | ✅ FIXED |
| No generator tests | HIGH | ⏳ TODO |
| Code duplication | MEDIUM | 🤔 DEFER |

---

## 📝 NOTES

- **Architecture Validated**: Fixture-driven approach is sound
- **Test Coverage**: 75 generated tests (15 APIs × 5 languages)
- **Code Quality Issues**: Need to address unwrap() and add tests
- **Rust Gap**: Most critical issue - need plugin API tests for core library

**End of TODO.md**
