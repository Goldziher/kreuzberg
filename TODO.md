# Kreuzberg V4 - Production Readiness TODO

**Branch**: v4-dev
**Last Updated**: 2025-10-18
**Phase**: Pre-Release - CI/CD and Production Validation
**Test Status**: 1,088+ tests passing ✅ (866 lib + 24 core + 207 integration)
**Coverage**: ~95%+ ✅

______________________________________________________________________

## 🚀 HIGH PRIORITY: CI/CD Validation & Updates

### 1. Rust Integration Tests in CI ⏳

**Goal**: Ensure all 207 Rust integration tests run in CI across all platforms

**Tasks**:

- [ ] Update `.github/workflows/ci.yaml` rust-tests job to run integration tests
    - Current: Only runs `cargo test --release --no-default-features`
    - Needed: Add integration test runs with features enabled
    - Add: `cargo test --release --features office --test '*_integration'`
- [ ] Configure Rust integration tests with proper feature flags
    - `office` feature for Pandoc tests
    - Handle optional dependencies (Tesseract, Pandoc, LibreOffice)
- [ ] Add integration test timeout configuration (currently 45min for rust-tests)
- [ ] Verify integration tests pass on all platforms (Ubuntu, macOS, Windows)

**Acceptance Criteria**:

- All 207 integration tests run in CI
- Tests pass on Ubuntu, macOS, Windows
- Proper feature flag configuration
- No flaky tests

______________________________________________________________________

### 2. Rust Coverage Validation ⏳

**Goal**: Validate Rust core achieves 95%+ coverage target in CI

**Tasks**:

- [ ] Review `cargo llvm-cov` output in coverage job (line 255-256)

    - Current: Runs with `--no-default-features`
    - Needed: Run with all features to include integration tests

- [ ] Update coverage job to test with features:

    ```bash
    cargo llvm-cov --features office --lcov --output-path rust-coverage.lcov
    ```

- [ ] Add coverage thresholds to CI (fail if < 95%)

- [ ] Generate coverage report showing breakdown by module

- [ ] Upload Rust coverage to DeepSource (already configured, line 263)

**Acceptance Criteria**:

- Rust coverage ≥ 95% in CI
- Coverage includes integration tests
- Coverage report uploaded to DeepSource
- CI fails if coverage drops below threshold

______________________________________________________________________

### 3. Update CI for v4 Architecture ⏳

**Goal**: Optimize CI for hybrid Rust core + Python bindings architecture

**Tasks**:

- [ ] Review and optimize rust-tests job:
    - Remove redundant tessdata inspection (lines 63-93)
    - Add Rust-specific caching (target/ directory)
    - Optimize test execution (parallel where possible)
- [ ] Update python-tests job categories:
    - Verify test categorization still matches v4 structure
    - Consider splitting Rust core tests vs Python binding tests
- [ ] Review coverage job matrix:
    - Currently tests Python 3.10, 3.13, 3.14 on all platforms
    - Verify Python 3.14 skips are correct (torch/paddle not available)
- [ ] Optimize caching strategy:
    - Add Rust target/ cache
    - Review Python dependency cache effectiveness
    - Add cargo registry cache
- [ ] Update validate job:
    - Ensure prek runs both Rust and Python checks
    - Add explicit Rust fmt/clippy with proper exit codes

**Acceptance Criteria**:

- CI runs efficiently (\<60min total)
- Proper caching reduces redundant work
- Clear separation of Rust vs Python test runs
- All checks enforce quality standards

______________________________________________________________________

### 4. Performance Benchmarks (v3 vs v4) ⏳

**Goal**: Demonstrate v4 performance improvements over v3

**Tasks**:

- [ ] Review existing benchmarks in `benchmarks/` directory
- [ ] Add v4-specific benchmarks:
    - Rust core extraction speed
    - Memory usage comparison (v3 vs v4)
    - Large file handling (10MB+, 100MB+ documents)
    - Batch processing throughput
- [ ] Update benchmark workflows:
    - `.github/workflows/kreuzberg-benchmarks.yaml`
    - `.github/workflows/comparative-benchmark.yaml`
- [ ] Create benchmark report showing:
    - Speed improvements (expected: 10-50x for text processing)
    - Memory efficiency gains
    - Throughput improvements for batch operations
- [ ] Document benchmarks in `benchmarks/README.md`

**Acceptance Criteria**:

- Benchmarks show measurable v4 improvements
- Automated benchmark runs in CI
- Results documented and shareable
- No performance regressions vs v3

______________________________________________________________________

## 📚 MEDIUM PRIORITY: Documentation & Release Prep

### 5. Update Documentation for v4 Architecture ⏳

**Goal**: Ensure docs reflect v4 Rust core architecture

**Tasks**:

- [ ] Review `docs/` directory structure
- [ ] Update architecture documentation:
    - Document Rust core modules (extraction/, core/, plugins/)
    - Explain PyO3 bindings (crates/kreuzberg-py)
    - Document plugin system (traits, registration)
- [ ] Update API documentation:
    - Rust API docs (`cargo doc`)
    - Python API docs (existing)
    - Migration guide (v3 → v4)
- [ ] Update installation docs:
    - Add Rust usage examples
    - Update Python installation (now includes Rust compilation)
    - Document feature flags (`office`, etc.)
- [ ] Update examples:
    - Add Rust usage examples
    - Update Python examples for v4 API
    - Add plugin development examples
- [ ] Review and update `CLAUDE.md`:
    - Ensure Rust conventions are accurate
    - Update architecture section
    - Add v4-specific patterns

**Acceptance Criteria**:

- Docs accurately reflect v4 architecture
- Clear migration path from v3
- Rust and Python examples work
- Docs build without warnings

______________________________________________________________________

### 6. Release Preparation (v4.0.0) ⏳

**Goal**: Prepare for v4.0.0 stable release

**Tasks**:

- [ ] Review and update `CHANGELOG.md`:
    - Document all v4 changes
    - List breaking changes vs v3
    - Highlight new features (Rust core, performance, plugins)
- [ ] Update version numbers:
    - `pyproject.toml` → 4.0.0
    - `Cargo.toml` (workspace + kreuzberg crate) → 4.0.0
    - `crates/kreuzberg-py/Cargo.toml` → 4.0.0
- [ ] Review `.github/workflows/release.yaml`:
    - Ensure wheel building works for v4 (Rust compilation)
    - Test sdist includes Rust source
    - Verify cibuildwheel config for Rust + Python
- [ ] Test release process on test.pypi.org:
    - Build wheels locally
    - Test installation on all platforms
    - Verify Rust bindings work
- [ ] Create release checklist:
    - All tests pass ✅
    - Coverage ≥ 95% ✅
    - Benchmarks show improvements ⏳
    - Docs updated ⏳
    - CHANGELOG complete ⏳
    - Version numbers updated ⏳

**Acceptance Criteria**:

- Release workflow tested and working
- All platforms build successfully
- Test installation works
- Ready for PyPI publish

______________________________________________________________________

### 7. Docker Images Update ⏳

**Goal**: Ensure Docker images work with v4 Rust architecture

**Tasks**:

- [ ] Review `.github/workflows/publish-docker.yaml`
- [ ] Test Docker builds locally:
    - Core variant (API + Tesseract)
    - EasyOCR variant
    - PaddleOCR variant
    - Vision-tables variant
    - All variant
- [ ] Update Dockerfile(s) if needed:
    - Ensure Rust toolchain installed
    - Optimize layer caching
    - Verify all features build correctly
- [ ] Test Docker images:
    - Run extraction tests in containers
    - Verify system dependencies (Tesseract, Pandoc, LibreOffice)
    - Check image sizes (should be reasonable)
- [ ] Update Docker documentation

**Acceptance Criteria**:

- All Docker variants build successfully
- Images tested and working
- Reasonable image sizes
- Documentation updated

______________________________________________________________________

## 🔍 LOW PRIORITY: Nice-to-Have Improvements

### 8. Additional Testing ⏳

**Goal**: Further increase test coverage and robustness

**Tasks**:

- [ ] Add property-based tests (hypothesis):
    - Text extraction invariants
    - MIME detection properties
    - Configuration validation
- [ ] Add fuzzing tests for parsers:
    - PDF parser
    - XML parser
    - Email parser
- [ ] Add stress tests:
    - Memory limits
    - Large file handling (1GB+)
    - Concurrent extraction limits
- [ ] Add integration tests for:
    - Python plugin registration
    - Cross-language plugin calls
    - Error propagation across FFI boundary

**Acceptance Criteria**:

- Additional test coverage
- No crashes on fuzzed inputs
- Documented stress test limits

______________________________________________________________________

### 9. Performance Profiling ⏳

**Goal**: Identify and document performance characteristics

**Tasks**:

- [ ] Profile Rust core with `cargo flamegraph`:
    - PDF extraction
    - XML streaming
    - Text processing
- [ ] Profile Python bindings overhead:
    - FFI call costs
    - Type conversion overhead
    - GIL impact
- [ ] Create performance guidelines:
    - When to use batch processing
    - Memory usage patterns
    - Optimal chunk sizes
- [ ] Document performance tips in docs

**Acceptance Criteria**:

- Flame graphs generated and analyzed
- Performance bottlenecks identified
- Guidelines documented

______________________________________________________________________

### 10. Security Audit ⏳

**Goal**: Ensure v4 is production-ready from security perspective

**Tasks**:

- [ ] Run `cargo audit` on Rust dependencies
- [ ] Review unsafe code blocks:
    - Verify all SAFETY comments are accurate
    - Consider eliminating unsafe where possible
- [ ] Test malicious inputs:
    - Zip bombs
    - XML billion laughs attack
    - PDF exploit attempts
    - Path traversal in archives
- [ ] Review error handling:
    - No panic on invalid input
    - No sensitive data in error messages
    - Proper sanitization of file paths
- [ ] Add security documentation:
    - Known limitations
    - Security best practices
    - Reporting vulnerabilities

**Acceptance Criteria**:

- No critical security issues
- Malicious inputs handled gracefully
- Security documentation complete

______________________________________________________________________

## 📊 Progress Summary

### Current Status

- ✅ **Integration Tests**: 207/207 complete (100%)
- ✅ **Test Coverage**: 95%+ achieved
- ✅ **Rust Core**: Complete and functional
- ✅ **Chunks Migration**: Complete (metadata → top-level field)
- ⏳ **CI/CD**: Needs updates for v4
- ⏳ **Documentation**: Needs v4 updates
- ⏳ **Release Prep**: Not started
- ⏳ **Rust-Native NLP**: Not started (fastembed-rs, gline-rs, keyword extraction)

### Priority Order

1. **Rust Integration Tests in CI** (HIGH) - Validate tests run correctly
2. **Rust Coverage Validation** (HIGH) - Ensure 95% maintained in CI
3. **Update CI for v4 Architecture** (HIGH) - Optimize CI workflows
4. **Performance Benchmarks** (HIGH) - Demonstrate improvements
5. **Update Documentation** (MEDIUM) - Reflect v4 changes
6. **Release Preparation** (MEDIUM) - Prepare for v4.0.0
7. **Docker Images Update** (MEDIUM) - Ensure containers work
8. **Additional Testing** (LOW) - Further hardening
9. **Performance Profiling** (LOW) - Optimization insights
10. **Security Audit** (LOW) - Production readiness
11. **fastembed-rs Integration** (MEDIUM) - Embeddings + reranking
12. **gline-rs Integration** (MEDIUM) - Named entity recognition
13. **Keyword Extraction** (MEDIUM) - YAKE or RAKE implementation
14. ~~**Chunks Field Migration**~~ (COMPLETE) ✅

### Estimated Timeline

- **High Priority Tasks (1-4)**: 2-3 days
- **Medium Priority Tasks (5-7, 11-13)**: 5-7 days
- **Low Priority Tasks (8-10)**: 2-4 days
- **Total to v4.0.0**: ~6-10 days (without NLP features)
- **Total to v4.1.0**: ~9-14 days (with NLP features)

______________________________________________________________________

## 🎯 Success Criteria for v4.0.0 Release

- ✅ All tests pass (1,088+ tests)
- ✅ Coverage ≥ 95% (both Rust and Python)
- ⏳ CI validates all platforms
- ⏳ Benchmarks show performance improvements
- ⏳ Documentation updated and accurate
- ⏳ Docker images tested and published
- ⏳ CHANGELOG complete
- ⏳ Release tested on test.pypi.org
- ⏳ No critical security issues
- ⏳ Migration guide available

______________________________________________________________________

## 🔧 MEDIUM PRIORITY: Rust-Native NLP Features

### 11. fastembed-rs Integration (Embeddings + Reranking) ⏳

**Goal**: Implement embeddings and reranking using Rust-native fastembed-rs for better performance

**Background**: fastembed-rs provides ONNX Runtime embeddings and reranking compatible with sentence-transformers models, eliminating ~500MB of Python dependencies (torch, transformers) and FFI overhead. Works with the new top-level `chunks` field.

**Tasks**:

- [ ] Add fastembed dependency to `Cargo.toml`:
    ```toml
    fastembed = "3.0"
    ```
- [ ] Create Rust embeddings module:
    - Create `crates/kreuzberg/src/embeddings/mod.rs`
    - Create `crates/kreuzberg/src/embeddings/config.rs` for EmbeddingConfig
    - Create `crates/kreuzberg/src/embeddings/engine.rs` for embedding and reranking
- [ ] Implement EmbeddingConfig struct:
    - `model: String` (default: "sentence-transformers/all-MiniLM-L12-v2")
    - `cache_dir: Option<PathBuf>` (default: system cache)
    - `batch_size: usize` (default: 32)
    - Derive Clone, Debug, Serialize, Deserialize
- [ ] Implement `create_embeddings()` async function:
    - Takes `chunks: Vec<String>` and `config: &EmbeddingConfig`
    - Returns `Result<Vec<Vec<f32>>>` (one embedding per chunk)
    - Uses fastembed's TextEmbedding API
    - Handles model initialization and caching
- [ ] Implement `rerank()` async function:
    - Takes `query: &str`, `documents: Vec<String>`, `config: &EmbeddingConfig`
    - Returns `Result<Vec<(usize, f32)>>` (document indices with scores)
    - Uses fastembed's reranking API
    - Useful for semantic search and retrieval
- [ ] Implement sync wrappers:
    - `create_embeddings_sync()` - Uses `tokio::runtime::Runtime::block_on()`
    - `rerank_sync()` - Uses `tokio::runtime::Runtime::block_on()`
- [ ] Add PyO3 bindings in `crates/kreuzberg-py/src/lib.rs`:
    - Expose embedding and reranking functions to Python
    - Convert Rust types to Python types (Vec<Vec<f32>> → List[List[float]])
    - Add proper error handling and type conversion
- [ ] Add Rust integration tests:
    - Test embedding creation with default config
    - Test custom model configuration
    - Test batch processing
    - Test reranking functionality
    - Test error cases (invalid model, empty input)
    - Test cache directory handling
- [ ] Add Python integration tests in `packages/python/tests/features/`:
    - Create `embeddings_test.py`
    - Test sync and async variants
    - Test integration with chunking
    - Test reranking with sample documents
    - Test custom configurations
    - Test error handling
- [ ] Update documentation:
    - Add embeddings section to Rust API docs
    - Document reranking use cases
    - Add Python examples
    - Document supported models
    - Add migration guide from sentence-transformers

**Acceptance Criteria**:

- fastembed-rs integrated and working in Rust core
- Both embedding and reranking functionality available
- PyO3 bindings expose functions to Python
- Tests pass on all platforms
- No torch/transformers dependencies required
- Documentation complete with examples
- Performance benchmarks show improvement over Python implementation

______________________________________________________________________

### 12. gline-rs Integration (Named Entity Recognition) ⏳

**Goal**: Implement Named Entity Recognition (NER) using Rust-native gline-rs

**Background**: gline-rs provides NER in Rust, eliminating the need for heavy spacy dependency while providing fast, accurate entity extraction.

**Tasks**:

- [ ] Add gline-rs dependency to `Cargo.toml`:
    ```toml
    gline = "0.1"  # Verify latest version
    ```
- [ ] Research gline-rs API and capabilities:
    - Supported entity types (PERSON, ORG, LOC, etc.)
    - Model loading and initialization
    - Language support
    - Performance characteristics
- [ ] Create Rust entity extraction module:
    - Create `crates/kreuzberg/src/ner/mod.rs`
    - Create `crates/kreuzberg/src/ner/config.rs`
    - Create `crates/kreuzberg/src/ner/engine.rs`
- [ ] Implement NerConfig struct:
    - `enabled: bool` (default: false)
    - `entity_types: Vec<String>` (default: all supported types)
    - `min_confidence: f32` (default: 0.5)
    - `language: String` (default: "en")
- [ ] Implement `extract_entities()` function:
    - Takes `text: &str` and `config: &NerConfig`
    - Returns `Result<Vec<Entity>>` where Entity has:
        - `text: String`
        - `entity_type: String`
        - `confidence: f32`
        - `start: usize`, `end: usize` (character offsets)
- [ ] Integrate with post-processing pipeline:
    - Add NER as PostProcessor plugin
    - Store results in `metadata.additional["entities"]`
    - Execute in Late stage (after chunking, quality processing)
- [ ] Add PyO3 bindings:
    - Expose entity extraction functions to Python
    - Convert Rust Entity struct to Python dict
    - Add proper error handling
- [ ] Remove spacy dependencies:
    - Check `packages/python/pyproject.toml` for spacy references
    - Remove any spacy-based entity extraction code
    - Update tests to use new Rust-based implementation
- [ ] Add integration tests:
    - Test entity extraction on sample text
    - Test configuration options
    - Test multiple languages (if supported)
    - Test integration with pipeline
    - Test overlapping entities
- [ ] Update documentation:
    - Document supported entity types
    - Add configuration examples
    - Add migration guide from spacy
    - Document performance characteristics

**Acceptance Criteria**:

- gline-rs integrated and working
- Entity extraction available as PostProcessor
- No spacy dependency required
- Tests pass on all platforms
- Documentation complete
- Performance benchmarks available

______________________________________________________________________

### 13. Keyword Extraction (YAKE + RAKE) ⏳ → ✅

**Goal**: Implement fast, multilingual keyword extraction with both YAKE and RAKE algorithms as optional features

**Background**: Keyword extraction helps identify important terms in documents. We'll support both algorithms as optional dependencies, letting users choose based on their needs:
- **YAKE**: Statistical approach, more advanced, weighs multiple factors (acronyms, position, capitalization, etc.)
- **RAKE**: Co-occurrence based, simpler and faster, good for quick keyword extraction

**Research Findings**:
- **yake-rust** (https://github.com/quesurifn/yake-rust) - MIT license, YAKE algorithm, language-agnostic statistical approach
- **rake** (https://github.com/yaa110/rake-rs) - MIT/Apache license, RAKE algorithm, multilingual support

**Implementation Status**: ✅ **CORE IMPLEMENTATION COMPLETE**

**Tasks**:

- [x] Add both as optional dependencies to `Cargo.toml`:
    ```toml
    [dependencies]
    yake-rust = { version = "0.1", optional = true }
    rake = { version = "x.y.z", optional = true }

    [features]
    stopwords = []  # New dedicated feature for stopwords
    keywords-yake = ["yake-rust", "stopwords"]
    keywords-rake = ["rake", "stopwords"]
    keywords = ["keywords-yake", "keywords-rake"]
    quality = ["...", "stopwords"]  # quality also uses stopwords
    ```
- [x] Create stopwords module (`crates/kreuzberg/src/stopwords/mod.rs`):
    - Extracted from `text/token_reduction/filters.rs`
    - Publicly accessible STOPWORDS lazy static
    - Supports English and Spanish (78+ and 250+ words)
    - JSON file loading capability
- [x] Create Rust keyword extraction module with unified interface:
    - Created `crates/kreuzberg/src/keywords/mod.rs`
    - Created `crates/kreuzberg/src/keywords/config.rs`
    - Created `crates/kreuzberg/src/keywords/yake.rs` (feature-gated)
    - Created `crates/kreuzberg/src/keywords/rake.rs` (feature-gated)
    - Created `crates/kreuzberg/src/keywords/types.rs` for shared types
- [x] Implement KeywordConfig struct:
    - `algorithm: KeywordAlgorithm` enum (Yake, Rake)
    - `max_keywords: usize` (default: 10)
    - `min_score: f32` (default: 0.0)
    - `ngram_range: (usize, usize)` (default: (1, 3) for unigrams to trigrams)
    - `language: Option<String>` (for stopword filtering)
    - `yake_params: Option<YakeParams>` (YAKE-specific tuning)
    - `rake_params: Option<RakeParams>` (RAKE-specific tuning)
- [x] Implement KeywordAlgorithm enum:
    ```rust
    pub enum KeywordAlgorithm {
        #[cfg(feature = "keywords-yake")]
        Yake,
        #[cfg(feature = "keywords-rake")]
        Rake,
    }
    ```
- [x] Implement unified `extract_keywords()` function:
    - Takes `text: &str` and `config: &KeywordConfig`
    - Returns `Result<Vec<Keyword>>` where Keyword has:
        - `text: String`
        - `score: f32`
        - `algorithm: KeywordAlgorithm` (which algo extracted it)
        - `positions: Option<Vec<usize>>` (where keyword appears)
    - Dispatches to appropriate backend based on config.algorithm
- [x] Implement YAKE backend (feature-gated):
    - Wrapper around yake-rust crate
    - Convert between yake-rust types and our Keyword type
    - Handle YAKE-specific configuration
    - Score normalization (lower scores are better → inverted to 0-1 range)
- [x] Implement RAKE backend (feature-gated):
    - Wrapper around rake crate
    - Convert between rake types and our Keyword type
    - Handle RAKE-specific configuration
    - Uses stopwords module for delimiter identification
- [x] Add unit tests (15 total, all passing):
    - RAKE: 6 tests (basic, min_score, ngram_range, empty text, custom params, multilingual)
    - YAKE: 5 tests (basic, min_score, ngram_range, empty text, custom params)
    - Module: 4 tests (default algorithm, YAKE explicit, RAKE explicit, algorithm comparison)
- [x] Add integration tests (20 total, all passing):
    - Created `tests/keywords_integration.rs`
    - RAKE: 10 tests (basic, max_keywords, min_score, ngram_range, Spanish, empty, short, domains, score distribution, struct properties)
    - YAKE: 10 tests (basic, max_keywords, min_score, ngram_range, Spanish, empty, short, domains, score distribution, struct properties)
    - YAKE vs RAKE comparison test
    - Tests with real documents (ML, climate change, Spanish)
    - Tests with various configurations (max_keywords, min_score, ngram_range)
    - Tests with multiple languages (English, Spanish)
    - All tests feature-gated and compile conditionally
- [x] Add quality evaluation tests (8 total, all passing):
    - Created `tests/keywords_quality.rs`
    - RAKE + YAKE quality tests with ground truth keywords
    - Precision/recall/F1 metrics for both algorithms
    - Default configs: F1 scores 0.38-0.59 (exceed 0.30 threshold)
    - Optimized configs: F1=0.75 (P=0.80, R=0.71)
    - Domain relevance tests (70-90% keywords contain relevant terms)
    - Validates both algorithms perform well out-of-the-box
- [x] Integrate with post-processing pipeline:
    - Created `KeywordExtractor` PostProcessor in `src/keywords/processor.rs`
    - Stores results in `metadata.additional["keywords"]`
    - Executes in Middle stage (after language detection, before late hooks)
    - Feature-gated: Only registered if keywords feature enabled
    - Added `ensure_initialized()` with lazy registration pattern
    - Added 8 processor unit tests (all passing)
    - Added 3 pipeline integration tests (all passing)
    - Validates processor only runs when config.keywords is set
    - Validates short content (<10 words) is skipped
    - Validates keywords are stored correctly in metadata
- [ ] Add PyO3 bindings:
    - Expose keyword extraction functions to Python
    - Expose algorithm selection enum
    - Convert Rust Keyword struct to Python dict
    - Add proper error handling
    - Feature-gated bindings (only expose if features enabled)
    - Test integration with pipeline
    - Feature-gated tests (conditional compilation)
    - Test with only one algorithm enabled
- [ ] Add benchmarks comparing YAKE vs RAKE:
    - Speed comparison
    - Memory usage
    - Quality comparison (precision/recall if ground truth available)
    - Document trade-offs
- [ ] Update documentation:
    - Document both YAKE and RAKE algorithms
    - Explain when to use which algorithm
    - Explain scoring mechanisms
    - Add configuration examples for both
    - Document feature flags
    - Document multilingual support
    - Add migration guide from Python keyword extraction

**Acceptance Criteria**:

- ✅ Both yake-rust and rake integrated as optional features
- ✅ Stopwords extracted into dedicated feature (used by keywords and quality)
- ✅ Unified KeywordConfig interface works with both algorithms
- ✅ Users can enable one or both via Cargo features
- ✅ All 15 unit tests pass (RAKE: 6, YAKE: 5, Module: 4)
- ✅ All 20 integration tests pass (RAKE: 10, YAKE: 10)
- ✅ All 8 quality evaluation tests pass (precision/recall/F1 metrics)
- ✅ Tests pass with both features enabled (20 tests)
- ✅ Tests pass with only one feature enabled (10 tests each)
- ✅ Tests with real documents (ML, climate change, Spanish)
- ✅ Tests with various configurations (max_keywords, min_score, ngram_range)
- ✅ Tests with multiple languages (English, Spanish)
- ✅ No compile errors when features disabled
- ✅ Default configs perform well (F1 ≥ 0.38, exceeds 0.30 threshold)
- ✅ PostProcessor plugin available (feature-gated)
- ✅ Pipeline integration complete with 11 tests (8 processor + 3 pipeline)
- ⏳ Benchmarks compare both algorithms
- ⏳ Documentation explains trade-offs and when to use each
- ⏳ PyO3 bindings expose functions to Python

______________________________________________________________________

### 14. Chunks Field Migration (Metadata → Top-Level) ✅

**Goal**: Make chunks a first-class citizen in ExtractionResult, not a metadata entry

**Status**: ✅ **RUST CORE COMPLETE** - All Rust-side work finished, Python bindings remain

**What Was Implemented** (Rust Core):

- ✅ `chunks: Option<Vec<String>>` field added to ExtractionResult (types.rs:29)
- ✅ Pipeline sets `result.chunks = Some(chunking_result.chunks)` (pipeline.rs:98)
- ✅ `chunk_count` kept in metadata for backward compatibility (pipeline.rs:101-106)
- ✅ Integration tests updated to validate `result.chunks` directly (config_features.rs)
- ✅ **All 15 extractors** updated to initialize `chunks: None`:
  - image.rs, archive.rs (ZIP/TAR/7Z), email.rs, excel.rs, html.rs
  - pandoc.rs, pdf.rs, pptx.rs, xml.rs, text.rs, structured.rs
  - core/extractor.rs, ocr/tesseract_backend.rs
- ✅ **All 6 plugin test files** fixed:
  - plugins/processor.rs, extractor.rs, registry.rs, validator.rs, mod.rs, ocr.rs
- ✅ **Code compiles successfully**: `cargo check --all-features` passes
- ✅ **Enhanced chunking tests**: Validates chunks field, chunk_count metadata, chunk size constraints

**Remaining Tasks** (Python Bindings - Optional):

- [ ] Update PyO3 bindings to expose chunks field to Python (crates/kreuzberg-py)
- [ ] Update Python type stubs (`_internal_bindings.pyi`)
- [ ] Update Python tests to use `result.chunks` instead of metadata
- [ ] Update documentation (Python API docs)

**Acceptance Criteria**:

- ✅ `chunks` field is top-level in ExtractionResult (not in metadata)
- ✅ Pipeline populates chunks field when chunking enabled
- ✅ `chunk_count` in metadata for backward compatibility
- ✅ All Rust extractors initialize chunks: None
- ✅ All plugin test files updated
- ✅ Integration tests validate result.chunks
- ✅ Code compiles with no errors
- ⏳ PyO3 bindings expose chunks field correctly (future work)
- ⏳ Python tests updated (future work)
- ⏳ Documentation updated (future work)

**Files Modified** (21 files total):
- Core: types.rs, pipeline.rs
- Extractors (15): image.rs, archive.rs×3, email.rs, excel.rs, html.rs, pandoc.rs, pdf.rs, pptx.rs, xml.rs, text.rs, structured.rs, extractor.rs, tesseract_backend.rs×2
- Plugins (6): processor.rs, extractor.rs, registry.rs, validator.rs, mod.rs, ocr.rs
- Tests: config_features.rs (enhanced validation)

______________________________________________________________________

## 📝 Notes

- Focus on HIGH priority tasks first - CI/CD validation is critical
- Documentation and release prep can happen in parallel
- Low priority tasks can be deferred to v4.1.0
- Keep an eye on CI execution time - optimize if > 60 minutes
- Test early and often on all platforms (Ubuntu, macOS, Windows)
- **Rust-native NLP features (11-13)** align with v4 architecture - consider for v4.1.0+
- ✅ **Chunks field migration (14) is COMPLETE** - all Rust-side work finished (21 files modified)
- fastembed-rs, gline-rs, and keyword extraction eliminate ~600MB+ of Python dependencies
- **Keyword extraction**: Both YAKE and RAKE supported as optional features, users choose via config
- All NLP features integrate as PostProcessor plugins in Late stage
- **Feature flags pattern**: Use Cargo features for optional NLP dependencies to keep binary size down
