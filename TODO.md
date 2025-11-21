# E2E Test System Overhaul + API Parity Gaps

**Generated**: 2025-11-21 (Updated)
**Branch**: `feature/close-all-api-gaps`
**Critical Issue Identified**: Hand-written E2E tests violating fixture-driven architecture

---

## 🚨 CRITICAL: Current State Analysis

### The Problem

**Hand-written E2E tests exist that claim to be "auto-generated"**:
- `e2e/python/tests/test_plugin_apis.py` - Header says "Auto-generated" but is hand-written
- `e2e/typescript/tests/plugin-apis.test.ts` - Header says "Auto-generated" but is hand-written
- `e2e/ruby/spec/plugin_apis_spec.rb` - Header says "Auto-generated" but is hand-written
- `e2e/java/src/test/java/dev/kreuzberg/e2e/PluginAPIsTest.java` - Header says "Auto-generated" but is hand-written
- `e2e/go/plugin_apis_test.go` - Header says "Auto-generated" but is hand-written

**E2E Generator (`tools/e2e-generator`) ONLY handles document extraction fixtures**:
- Uses fixtures from `fixtures/` (PDF, Office, HTML, etc.)
- Does NOT generate plugin API tests
- No fixture schema for plugin/config/utility API tests

**This violates core architecture**: E2E tests MUST be generated from fixtures, not hand-maintained.

---

## Phase 1: Remove Hand-Written Tests & Fix Architecture

### Step 1.1: Audit Current Hand-Written Tests

**Files to inspect**:
- [ ] `e2e/python/tests/test_plugin_apis.py` - What APIs does it test?
- [ ] `e2e/typescript/tests/plugin-apis.test.ts` - What APIs does it test?
- [ ] `e2e/ruby/spec/plugin_apis_spec.rb` - What APIs does it test?
- [ ] `e2e/java/src/test/java/dev/kreuzberg/e2e/PluginAPIsTest.java` - What APIs does it test?
- [ ] `e2e/go/plugin_apis_test.go` - What APIs does it test?

**Create matrix**: Which plugin/config/utility APIs are currently tested by hand-written tests?

### Step 1.2: Design Fixture Schema for Plugin API Tests

**New fixture type**: `plugin_api_fixtures/` or extend existing schema

**Categories needed**:
1. **Validator Management**:
   - `list_validators`
   - `register_validator` (if applicable)
   - `unregister_validator` (if applicable)
   - `clear_validators`

2. **Post-Processor Management**:
   - `list_post_processors`
   - `register_post_processor` (if applicable)
   - `unregister_post_processor` (if applicable)
   - `clear_post_processors`

3. **OCR Backend Management**:
   - `list_ocr_backends`
   - `register_ocr_backend` (if applicable)
   - `unregister_ocr_backend`
   - `clear_ocr_backends`

4. **Document Extractor Management**:
   - `list_document_extractors`
   - `register_document_extractor` (NEW - needs implementation)
   - `unregister_document_extractor`
   - `clear_document_extractors`

5. **Configuration Loading**:
   - `ExtractionConfig.from_file()` / `fromFile()` / `ConfigFromFile()`
   - `ExtractionConfig.discover()` / `discover()` / `ConfigDiscover()`

6. **MIME Utilities**:
   - `detect_mime_type(bytes)` / `detectMimeType(bytes)`
   - `detect_mime_type_from_path(path)` / `detectMimeTypeFromPath(path)`
   - `get_extensions_for_mime(mime_type)` / `getExtensionsForMime(mimeType)`
   - `validate_mime_type(mime_type)` / `validateMimeType(mimeType)`

7. **Embedding Presets** (if applicable):
   - `list_embedding_presets()` / `listEmbeddingPresets()`
   - `get_embedding_preset(name)` / `getEmbeddingPreset(name)`

**Fixture Schema Design**:

```json
{
  "id": "list_validators",
  "category": "plugin_api",
  "api_type": "validator_management",
  "description": "List all registered validators",
  "test_spec": {
    "function": "list_validators",
    "args": [],
    "assertions": {
      "returns_list": true,
      "list_item_type": "string"
    }
  }
}
```

```json
{
  "id": "config_from_file_basic",
  "category": "config_api",
  "api_type": "config_loading",
  "description": "Load configuration from TOML file",
  "test_spec": {
    "function": "ExtractionConfig.from_file",
    "setup": {
      "create_temp_file": true,
      "file_content": "[chunking]\nmax_chars = 100\nmax_overlap = 20\n"
    },
    "args": ["${temp_file_path}"],
    "assertions": {
      "returns_config": true,
      "config_has_chunking": true,
      "chunking.max_chars": 100,
      "chunking.max_overlap": 20
    }
  }
}
```

**Action Items**:
- [ ] Design complete fixture schema for plugin/config/utility APIs
- [ ] Create `fixtures/plugin_api/` directory
- [ ] Document schema in `fixtures/plugin_api/schema.json`

### Step 1.3: Extend E2E Generator

**Files to modify**:
- `tools/e2e-generator/src/fixtures.rs` - Extend `Fixture` struct to handle plugin API fixtures
- `tools/e2e-generator/src/python.rs` - Add plugin API test generation
- `tools/e2e-generator/src/typescript.rs` - Add plugin API test generation
- `tools/e2e-generator/src/ruby.rs` - Add plugin API test generation
- `tools/e2e-generator/src/java.rs` - Add plugin API test generation
- `tools/e2e-generator/src/go.rs` - Add plugin API test generation
- `tools/e2e-generator/src/rust.rs` - Add plugin API test generation (if needed)

**Generator Changes**:
1. Parse plugin API fixtures from `fixtures/plugin_api/`
2. Generate test functions for each API category
3. Handle language-specific naming conventions (snake_case vs camelCase vs PascalCase)
4. Generate assertions based on fixture spec
5. Handle temp file creation for config loading tests

**Action Items**:
- [ ] Extend `Fixture` struct to support `api_type` and `test_spec` fields
- [ ] Add plugin API test generation to Python generator
- [ ] Add plugin API test generation to TypeScript generator
- [ ] Add plugin API test generation to Ruby generator
- [ ] Add plugin API test generation to Java generator
- [ ] Add plugin API test generation to Go generator
- [ ] Test generator with sample fixtures

### Step 1.4: Generate New Tests & Remove Hand-Written

**Process**:
1. Run generator for each language: `cargo run -p kreuzberg-e2e-generator -- generate --lang <language>`
2. Compare generated tests with hand-written tests
3. Verify generated tests cover all cases
4. Delete hand-written test files
5. Update `.gitignore` to prevent hand-written E2E tests

**Action Items**:
- [ ] Generate Python plugin API tests from fixtures
- [ ] Generate TypeScript plugin API tests from fixtures
- [ ] Generate Ruby plugin API tests from fixtures
- [ ] Generate Java plugin API tests from fixtures
- [ ] Generate Go plugin API tests from fixtures
- [ ] Verify generated tests compile
- [ ] Run generated tests (expect some failures - that's RED phase)
- [ ] Delete hand-written files:
  - `e2e/python/tests/test_plugin_apis.py`
  - `e2e/typescript/tests/plugin-apis.test.ts`
  - `e2e/ruby/spec/plugin_apis_spec.rb`
  - `e2e/java/src/test/java/dev/kreuzberg/e2e/PluginAPIsTest.java`
  - `e2e/go/plugin_apis_test.go`
- [ ] Add to `.gitignore`: `e2e/**/tests/**/*plugin*api*` (or similar pattern)
- [ ] Commit: "refactor: replace hand-written plugin API tests with generated tests"

---

## Phase 2: Implement Missing APIs (TDD - RED → GREEN)

Now that we have **generated** tests, we can see which APIs are missing (RED phase).

### Step 2.1: Run Generated Tests (RED Phase)

**Action Items**:
- [ ] Run Python generated tests → identify missing APIs
- [ ] Run TypeScript generated tests → identify missing APIs
- [ ] Run Ruby generated tests → identify missing APIs
- [ ] Run Java generated tests → identify missing APIs
- [ ] Run Go generated tests → identify missing APIs
- [ ] Create matrix of missing APIs per language

### Step 2.2: Implement Missing APIs (GREEN Phase)

**Priority Order** (from API parity review):

#### P0: Critical - Missing in ALL Bindings
- [ ] **`register_document_extractor()`**
  - Python: `crates/kreuzberg-py/src/plugins.rs`, `packages/python/kreuzberg/__init__.py`
  - TypeScript: `crates/kreuzberg-node/src/lib.rs`, `packages/typescript/src/index.ts`
  - Ruby: `packages/ruby/ext/kreuzberg_rb/native/src/lib.rs`, `packages/ruby/lib/kreuzberg.rb`
  - Java: `crates/kreuzberg-ffi/src/lib.rs`, `packages/java/src/main/java/dev/kreuzberg/Kreuzberg.java`
  - Go: `packages/go/kreuzberg/plugins.go`
  - **Note**: Requires implementing trait wrapper for language-specific extractor objects

#### P1: High - Config Loading APIs
- [ ] **Python: `ExtractionConfig.from_file()` and `ExtractionConfig.discover()`**
  - File: `crates/kreuzberg-py/src/config.rs`
  - Add class methods to `ExtractionConfig`

- [ ] **Ruby: `Config::Extraction.from_file` and `Config::Extraction.discover`**
  - File: `packages/ruby/ext/kreuzberg_rb/native/src/lib.rs`
  - Add class methods

- [ ] **Rust Core: Export post-processor mutation APIs**
  - File: `crates/kreuzberg/src/plugins/mod.rs`
  - Export: `register_post_processor`, `unregister_post_processor`, `clear_post_processors`

#### P2: Medium - Ruby Missing APIs
- [ ] **Ruby: MIME Utilities (4 APIs)**
  - `detect_mime_type(data)`
  - `detect_mime_type_from_path(path)`
  - `get_extensions_for_mime(mime_type)`
  - `validate_mime_type(mime_type)`
  - File: `packages/ruby/ext/kreuzberg_rb/native/src/lib.rs`, `packages/ruby/lib/kreuzberg.rb`

- [ ] **Ruby: Embedding Presets (2 APIs)**
  - `list_embedding_presets()`
  - `get_embedding_preset(name)`
  - File: `packages/ruby/ext/kreuzberg_rb/native/src/lib.rs`, `packages/ruby/lib/kreuzberg.rb`

- [ ] **Ruby: `unregister_document_extractor()`**
  - Likely already implemented in native binding
  - Just needs export in `packages/ruby/lib/kreuzberg.rb`

#### P3: Low - Python Missing API
- [ ] **Python: `validate_mime_type()`**
  - File: `crates/kreuzberg-py/src/lib.rs`, `packages/python/kreuzberg/__init__.py`

### Step 2.3: Verify Tests Pass (GREEN Phase)

**Action Items**:
- [ ] Run Python generated tests → 100% passing
- [ ] Run TypeScript generated tests → 100% passing
- [ ] Run Ruby generated tests → 100% passing
- [ ] Run Java generated tests → 100% passing
- [ ] Run Go generated tests → 100% passing
- [ ] Run `cargo clippy --all-targets --all-features` → zero warnings
- [ ] Commit implementations with passing tests

---

## Phase 3: Create Fixtures for Missing API Coverage

**If generated tests don't cover all missing APIs**, create fixtures:

### New Fixtures Needed

**From API Parity Review**:
- [ ] `fixtures/plugin_api/register_document_extractor.json` - NEW API, needs fixtures
- [ ] `fixtures/config_api/config_from_file.json` - Config loading
- [ ] `fixtures/config_api/config_discover.json` - Config discovery
- [ ] `fixtures/mime_api/detect_mime_from_bytes.json` - MIME detection from bytes
- [ ] `fixtures/mime_api/detect_mime_from_path.json` - MIME detection from path
- [ ] `fixtures/mime_api/get_extensions_for_mime.json` - Extension lookup
- [ ] `fixtures/mime_api/validate_mime_type.json` - MIME validation
- [ ] `fixtures/embedding_api/list_presets.json` - Embedding preset list
- [ ] `fixtures/embedding_api/get_preset.json` - Embedding preset get

**Process**:
1. Create fixture JSON files
2. Regenerate tests: `cargo run -p kreuzberg-e2e-generator -- generate --lang <language>`
3. Implement APIs
4. Verify tests pass

---

## Phase 4: Documentation & Cleanup

### Step 4.1: Document Generator System

**Action Items**:
- [ ] Update `tools/e2e-generator/README.md` with plugin API fixture docs
- [ ] Document fixture schema in `fixtures/plugin_api/schema.json`
- [ ] Add examples to `fixtures/plugin_api/examples/`
- [ ] Update main `README.md` to explain E2E test generation

### Step 4.2: Add CI Checks

**Action Items**:
- [ ] Add CI step to verify E2E tests are generated (not hand-written)
- [ ] Add CI step to regenerate tests and check for git diff
- [ ] Document regeneration process in `CONTRIBUTING.md`

### Step 4.3: Final Verification

**Action Items**:
- [ ] All E2E tests are generated from fixtures
- [ ] No hand-written E2E tests remain
- [ ] All language bindings have 100% API parity
- [ ] All generated tests pass
- [ ] Clippy passes
- [ ] Documentation updated
- [ ] Remove TODO.md
- [ ] Create PR

---

## Implementation Checklist Summary

### Phase 1: Fix Architecture (CRITICAL)
- [ ] 1.1: Audit hand-written tests
- [ ] 1.2: Design plugin API fixture schema
- [ ] 1.3: Extend E2E generator to support plugin APIs
- [ ] 1.4: Generate tests, delete hand-written files

### Phase 2: Implement APIs (TDD)
- [ ] 2.1: Run generated tests (RED)
- [ ] 2.2: Implement missing APIs (GREEN)
- [ ] 2.3: Verify tests pass

### Phase 3: Coverage
- [ ] 3.1: Create fixtures for any remaining gaps
- [ ] 3.2: Regenerate and verify

### Phase 4: Cleanup
- [ ] 4.1: Documentation
- [ ] 4.2: CI checks
- [ ] 4.3: Final verification

---

## Expected Final State

1. **Zero hand-written E2E tests** - All generated from fixtures
2. **100% API parity** across all 5 language bindings
3. **Fixtures cover**:
   - Document extraction (existing)
   - Plugin management APIs (new)
   - Configuration APIs (new)
   - MIME utilities (new)
   - Embedding presets (new)
4. **Generator handles** all test generation
5. **CI enforces** fixture-driven architecture

---

## Notes

- **DO NOT** write any E2E tests by hand
- **DO NOT** modify generated test files directly
- **ALWAYS** work through fixtures and generator
- Generator is source of truth for E2E tests
- This is non-negotiable architecture

**End of TODO.md**
