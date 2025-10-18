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
- ⏳ **CI/CD**: Needs updates for v4
- ⏳ **Documentation**: Needs v4 updates
- ⏳ **Release Prep**: Not started

### Priority Order

1. **Rust Integration Tests in CI** (HIGH) - Validate tests run correctly
1. **Rust Coverage Validation** (HIGH) - Ensure 95% maintained in CI
1. **Update CI for v4 Architecture** (HIGH) - Optimize CI workflows
1. **Performance Benchmarks** (HIGH) - Demonstrate improvements
1. **Update Documentation** (MEDIUM) - Reflect v4 changes
1. **Release Preparation** (MEDIUM) - Prepare for v4.0.0
1. **Docker Images Update** (MEDIUM) - Ensure containers work
1. **Additional Testing** (LOW) - Further hardening
1. **Performance Profiling** (LOW) - Optimization insights
1. **Security Audit** (LOW) - Production readiness

### Estimated Timeline

- **High Priority Tasks**: 2-3 days
- **Medium Priority Tasks**: 2-3 days
- **Low Priority Tasks**: 2-4 days
- **Total**: ~6-10 days to v4.0.0 release

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

## 📝 Notes

- Focus on HIGH priority tasks first - CI/CD validation is critical
- Documentation and release prep can happen in parallel
- Low priority tasks can be deferred to v4.1.0
- Keep an eye on CI execution time - optimize if > 60 minutes
- Test early and often on all platforms (Ubuntu, macOS, Windows)
