# Kreuzberg V4 - Concrete Development Tasks

**Status**: Phase 4 - FFI Bridge & Integration Testing
**Last Updated**: 2025-10-16 (Evening)
**Test Status**: 839 Rust tests + 56/71 Python tests passing
**Coverage**: ~92-94% (target: 95%)
**Code Quality**: 10/10 ✅

## 📋 Active Task Queue (In Priority Order)

### Task 1: Fix Failing PostProcessor Tests (30-60 minutes) ⭐ NEXT

**Status**: 12/71 tests failing (minor edge cases)
**Priority**: P0 - Blocks postprocessor completion
**Files**: `packages/python/tests/postprocessors/`

#### Subtasks

1. **Fix ImportError Mock Tests (3 tests)**

    - **Files**:
        - `test_entity_extraction.py:234` - `test_initialize_without_spacy`
        - `test_keyword_extraction.py:231` - `test_initialize_without_keybert`
        - `test_category_extraction.py:350` - `test_initialize_without_transformers`
    - **Issue**: Tests mock missing dependencies but deps are installed
    - **Solution**: Skip these tests or use better mocking that actually hides the modules
    - **Acceptance**: All 3 tests either pass or are properly skipped

1. **Fix Entity Extraction Tests (5 tests)**

    - **Files**: `test_entity_extraction.py`
        - Line 80: `test_process_empty_content`
        - Line 96: `test_process_missing_content`
        - Line 113: `test_process_non_string_content`
        - Line 276: `test_metadata_not_overwritten`
    - **Issue**: Tests fail because spaCy model `en_core_web_sm` not downloaded
    - **Solution**:
        - Add `pytest.mark.skipif` decorator checking if model is available
        - Use `spacy.util.is_package("en_core_web_sm")` check
    - **Acceptance**: Tests either pass with model or skip gracefully

1. **Fix Auto-Registration Test (1 test)**

    - **File**: `test_auto_registration.py:29` - `test_entity_extraction_registration_with_spacy`
    - **Issue**: Entity extraction not registered (spaCy model missing)
    - **Solution**: Skip if spaCy model not available
    - **Acceptance**: Test passes or skips based on model availability

1. **Fix FFI Bridge Error Handling Tests (2 tests)**

    - **Files**: `test_ffi_bridge.py`
        - Line 225: `test_unregister_nonexistent_processor`
        - Line 236: `test_register_duplicate_processor_name`
    - **Issue**: Tests expect RuntimeError but registry doesn't raise on these cases
    - **Solution**:
        - Check Rust `unregister_post_processor` and `register_post_processor` implementations
        - Verify they raise PyRuntimeError for these cases
        - If not, update tests to match actual behavior or fix Rust implementation
    - **Acceptance**: Tests match actual Rust FFI behavior

1. **Fix Processor Failure Handling Tests (2 tests)**

    - **Files**:
        - `test_keyword_extraction.py:295` - `test_extraction_failure_handling`
        - `test_category_extraction.py:418` - `test_classification_failure_handling`
    - **Issue**: Tests set `_extractor = None` but processor still returns results
    - **Solution**: Update processors to handle `None` models gracefully (return empty results)
    - **Acceptance**: Processors return minimal/empty results when model is None

**Completion Criteria**:

- ✅ 68-71/71 tests passing
- ✅ All failures either fixed or have clear skip conditions
- ✅ No flaky tests

**Estimated Time**: 30-60 minutes

______________________________________________________________________

### Task 2: Integration with Extraction Pipeline (1-2 hours)

**Status**: Not started
**Priority**: P1 - Core feature completion
**Goal**: Make postprocessors run automatically during extraction

#### Approach Decision

**Recommended**: Option B (Python-layer postprocessing)

- Keep Rust extraction pure and fast
- Python wrapper adds optional enrichment
- Easier to debug and test
- Better separation of concerns

#### Subtasks

1. **Create Python Extraction Wrapper (30 min)**

    - **File**: `packages/python/kreuzberg/extraction.py` (new file)
    - **Implementation**:

        ```python
        from typing import Optional
        from kreuzberg._internal_bindings import (
            extract_file as _extract_file_rust,
            extract_bytes as _extract_bytes_rust,
            ExtractionConfig,
        )
        from kreuzberg import list_post_processors

        async def extract_file(
            path: str, mime_type: Optional[str] = None, config: Optional[ExtractionConfig] = None, enable_postprocessors: bool = True
        ):
            # Call Rust extraction
            result = await _extract_file_rust(path, mime_type, config)

            # Apply postprocessors if enabled
            if enable_postprocessors:
                result = await _apply_postprocessors(result, config)

            return result

        async def _apply_postprocessors(result, config):
            # Get registered processors
            # Call each in order (early → middle → late)
            # Return enriched result
            ...
        ```

    - **Acceptance**: Wrapper functions work, call Rust + processors

1. **Add Configuration Support (20 min)**

    - **File**: Extend `ExtractionConfig` or create `PostProcessorConfig`
    - **Options**:
        - `enable_postprocessors: bool = True`
        - `enabled_processors: Optional[list[str]] = None` (whitelist)
        - `disabled_processors: Optional[list[str]] = None` (blacklist)
    - **Acceptance**: Config controls which processors run

1. **Implement Processor Ordering (20 min)**

    - **Logic**: Call processors in stages (early → middle → late)
    - **Files**:
        - `packages/python/kreuzberg/extraction.py`
    - **Implementation**:

        ```python
        from kreuzberg._internal_bindings import list_post_processors

        # Group processors by stage
        early = []  # entity extraction
        middle = []  # keywords, categories
        late = []  # custom user processors

        # Call in order, passing result through pipeline
        ```

    - **Acceptance**: Processors execute in correct order

1. **Update `__init__.py` Exports (10 min)**

    - **File**: `packages/python/kreuzberg/__init__.py`
    - **Changes**:
        - Export new wrapper functions alongside Rust functions
        - Add deprecation notice for direct Rust function usage (optional)
    - **Acceptance**: Users can import wrapper functions

1. **Add Tests (20 min)**

    - **File**: `packages/python/tests/test_extraction_pipeline.py` (new)
    - **Tests**:
        - Test extraction with postprocessors enabled
        - Test extraction with postprocessors disabled
        - Test selective processor enabling
        - Test processor ordering
        - Verify metadata enrichment
    - **Acceptance**: 10+ integration tests passing

**Completion Criteria**:

- ✅ Python wrapper functions available
- ✅ Postprocessors run automatically on extraction
- ✅ Configuration controls processor execution
- ✅ Tests verify end-to-end pipeline
- ✅ Documentation updated

**Estimated Time**: 1-2 hours

______________________________________________________________________

### Task 3: Migrate OCR Backends from Legacy (3-4 hours)

**Status**: FFI bridge 100% complete, migration 0%
**Priority**: P1 - Unblock Python OCR functionality
**Goal**: Migrate EasyOCR and PaddleOCR from `_legacy/_ocr/` to new FFI bridge

#### Subtasks

1. **Create OCR Protocol Interface (15 min)**

    - **File**: `packages/python/kreuzberg/ocr/protocol.py` (new)
    - **Content**:

        ```python
        from typing import Protocol

        class OcrBackendProtocol(Protocol):
            """Protocol for OCR backends compatible with Rust FFI bridge."""

            def name(self) -> str:
                """Return backend name (e.g., 'easyocr', 'paddleocr')."""
                ...

            def supported_languages(self) -> list[str]:
                """Return list of supported language codes."""
                ...

            def process_image(self, image_bytes: bytes, language: str) -> dict:
                """Process image bytes and return extraction result dict.

                Args:
                    image_bytes: Raw image data (PNG, JPEG, etc.)
                    language: Language code (e.g., 'eng', 'chi_sim')

                Returns:
                    {
                        "content": "extracted text",
                        "metadata": {"confidence": 0.95, ...},
                        "tables": []  # optional
                    }
                """
                ...
            # Optional lifecycle methods
            def initialize(self) -> None: ...
            def shutdown(self) -> None: ...
            def version(self) -> str: ...
        ```

    - **Acceptance**: Protocol defined, type-checked

1. **Migrate EasyOCR Backend (1.5 hours)**

    - **Source**: `/kreuzberg/_legacy/_ocr/_easyocr.py` (504 lines)
    - **Target**: `packages/python/kreuzberg/ocr/easyocr.py` (new)
    - **Steps**:
        a. Copy legacy EasyOCRBackend class
        b. Adapt to protocol interface:
        - Implement `name()` → return `"easyocr"`
        - Implement `supported_languages()` → return language list
        - Implement `process_image(bytes, lang)`:
            - Convert bytes → PIL Image
            - Call internal OCR method
            - Convert result → dict format
                c. Keep all features:
        - Device selection (CPU/GPU)
        - Language model loading
        - Result caching (if present)
        - Confidence scores
            d. Add proper error handling
            e. Add docstrings
    - **Acceptance**:
        - EasyOCRBackend class implements protocol
        - All legacy functionality preserved
        - Can be registered with `register_ocr_backend()`

1. **Migrate PaddleOCR Backend (1.5 hours)**

    - **Source**: `/kreuzberg/_legacy/_ocr/_paddleocr.py` (417 lines)
    - **Target**: `packages/python/kreuzberg/ocr/paddleocr.py` (new)
    - **Steps**: (same as EasyOCR)
        a. Copy legacy PaddleBackend class
        b. Adapt to protocol interface
        c. Keep all features (device, caching, etc.)
        d. Add error handling and docs
    - **Acceptance**: PaddleOCRBackend implements protocol

1. **Create Auto-Registration Module (20 min)**

    - **File**: `packages/python/kreuzberg/ocr/__init__.py` (new)
    - **Content**:

        ```python
        """Python OCR backends that register with Rust core."""

        from kreuzberg import register_ocr_backend

        # Track what's been registered
        __all__ = []
        __registered = set()

        def _register_backends():
            """Auto-register available OCR backends."""

            # Try EasyOCR
            try:
                from .easyocr import EasyOCRBackend

                backend = EasyOCRBackend()
                register_ocr_backend(backend)
                __all__.append("EasyOCRBackend")
                __registered.add("easyocr")
            except ImportError:
                pass  # easyocr not installed

            # Try PaddleOCR
            try:
                from .paddleocr import PaddleOCRBackend

                backend = PaddleOCRBackend()
                register_ocr_backend(backend)
                __all__.append("PaddleOCRBackend")
                __registered.add("paddleocr")
            except ImportError:
                pass  # paddleocr not installed

        # Auto-register on import
        _register_backends()
        ```

    - **Acceptance**: Import triggers registration

1. **Update Main Package Init (5 min)**

    - **File**: `packages/python/kreuzberg/__init__.py`
    - **Add**:

        ```python
        # Auto-register Python OCR backends (if installed)
        try:
            from . import ocr  # Triggers registration
        except ImportError:
            pass  # Optional OCR backends not available
        ```

    - **Acceptance**: Importing `kreuzberg` auto-registers OCR backends

1. **Add Integration Tests (30 min)**

    - **File**: `packages/python/tests/ocr/test_easyocr_integration.py` (new)
    - **File**: `packages/python/tests/ocr/test_paddleocr_integration.py` (new)
    - **Tests**:
        - Test backend registration
        - Test appears in `list_ocr_backends()`
        - Test extraction with OCR backend via Rust API
        - Test unregistration
        - Test error handling
    - **Acceptance**: 10+ tests per backend

**Completion Criteria**:

- ✅ EasyOCR and PaddleOCR migrated and working
- ✅ Backends auto-register on import
- ✅ Can be used via Rust extraction functions
- ✅ All legacy functionality preserved
- ✅ Integration tests passing
- ✅ Legacy `_ocr/` directory can be deleted

**Estimated Time**: 3-4 hours

______________________________________________________________________

### Task 4: Add MCP and API CLI Commands (2-3 hours)

**Status**: Not started
**Priority**: P2 - Nice to have for v4.0
**Goal**: Expose Rust MCP and API servers via CLI commands

#### Subtasks

1. **Add MCP Command to Python CLI (30 min)**

    - **File**: `packages/python/kreuzberg/cli.py`
    - **Implementation**:

        ```python
        @click.command()
        @click.option("--transport", type=click.Choice(["stdio"]), default="stdio")
        def mcp(transport: str):
            """Start the MCP (Model Context Protocol) server."""
            import subprocess
            import shutil

            # Check if rust binary has MCP support
            rust_binary = shutil.which("kreuzberg")
            if not rust_binary:
                click.echo("Error: Rust binary 'kreuzberg' not found in PATH")
                return 1

            # Try to start MCP server
            try:
                result = subprocess.run([rust_binary, "mcp", "--transport", transport], check=True)
                return result.returncode
            except FileNotFoundError:
                click.echo("Error: MCP feature not available")
                click.echo("Rebuild Rust binary with: cargo build --features mcp")
                return 1
        ```

    - **Acceptance**: Python CLI can start Rust MCP server

1. **Add MCP Command to Rust CLI (1 hour)**

    - **File**: `crates/kreuzberg-cli/src/main.rs`
    - **Changes**:

        ```rust
        #[derive(Subcommand)]
        enum Commands {
            // ... existing commands ...

            /// Start MCP (Model Context Protocol) server
            #[cfg(feature = "mcp")]
            Mcp {
                /// Transport type (stdio only for now)
                #[arg(long, default_value = "stdio")]
                transport: String,
            },
        }

        async fn main() -> Result<()> {
            match cli.command {
                // ... existing handlers ...

                #[cfg(feature = "mcp")]
                Commands::Mcp { transport } => {
                    if transport != "stdio" {
                        eprintln!("Only stdio transport is currently supported");
                        std::process::exit(1);
                    }

                    // Start MCP server
                    kreuzberg::mcp::start_server().await?;
                }
            }
        }
        ```

    - **Acceptance**: `kreuzberg mcp` starts MCP server

1. **Add Serve Command to Rust CLI (1 hour)**

    - **File**: `crates/kreuzberg-cli/src/main.rs`
    - **Changes**:

        ```rust
        #[derive(Subcommand)]
        enum Commands {
            // ... existing commands ...

            /// Start HTTP API server
            #[cfg(feature = "api")]
            Serve {
                /// Host to bind to
                #[arg(long, default_value = "127.0.0.1")]
                host: String,

                /// Port to listen on
                #[arg(long, default_value = "3000")]
                port: u16,
            },
        }

        async fn main() -> Result<()> {
            match cli.command {
                // ... existing handlers ...

                #[cfg(feature = "api")]
                Commands::Serve { host, port } => {
                    println!("Starting API server on {}:{}", host, port);
                    kreuzberg::api::start_server(&host, port).await?;
                }
            }
        }
        ```

    - **Acceptance**: `kreuzberg serve` starts API server

1. **Update CLI Documentation (20 min)**

    - **Files**:
        - Update help text in CLI
        - Add examples to README
        - Document feature flags needed
    - **Acceptance**: Users can discover and use new commands

1. **Add CLI Tests (10 min)**

    - **File**: `crates/kreuzberg-cli/tests/cli_tests.rs`
    - **Tests**:
        - Test `kreuzberg mcp --help`
        - Test `kreuzberg serve --help`
        - Test invalid options
    - **Acceptance**: Help text shows correctly

**Completion Criteria**:

- ✅ Python CLI can proxy to Rust MCP server
- ✅ Rust CLI has `mcp` command (with feature gate)
- ✅ Rust CLI has `serve` command (with feature gate)
- ✅ Documentation updated
- ✅ Help text clear and accurate

**Estimated Time**: 2-3 hours

______________________________________________________________________

### Task 5: Comprehensive Rust Integration Tests with Extensive OCR (4-6 hours) 🔥

**Status**: Not started
**Priority**: P1 - Critical for production readiness
**Goal**: Add extensive integration tests to Rust crate covering real OCR workflows

#### Context

Current state:

- 839 Rust unit tests passing
- OCR backend registry working
- Python OCR FFI bridge complete
- **Missing**: Real-world integration tests with actual OCR backends

#### Subtasks

1. **Set Up Test Infrastructure (30 min)**

    - **File**: `crates/kreuzberg/tests/common/mod.rs` (new)
    - **Setup**:
        - Test fixtures for sample images (PNG, JPEG, TIFF)
        - OCR result validation helpers
        - Mock OCR backend for testing
        - Test image generation utilities
    - **Test Images Needed**:
        - Simple text (single line)
        - Multi-line text
        - Mixed languages (English, Chinese, Arabic)
        - Low quality scans
        - Rotated text
        - Tables with text
        - Handwritten text
    - **Acceptance**: Test infrastructure ready

1. **OCR Backend Registry Tests (45 min)**

    - **File**: `crates/kreuzberg/tests/ocr_registry_tests.rs` (new)
    - **Tests**:
        - Test backend registration
        - Test duplicate registration error
        - Test backend listing
        - Test backend removal
        - Test thread-safe concurrent access
        - Test plugin lifecycle (initialize/shutdown)
        - Test backend selection by name
        - Test language support queries
    - **Acceptance**: 10+ tests covering registry operations

1. **Tesseract OCR Integration Tests (1 hour)**

    - **File**: `crates/kreuzberg/tests/tesseract_integration_tests.rs` (new)
    - **Prerequisites**: Tesseract must be installed on system
    - **Tests**:
        - **Basic text extraction**:
            - Test single-line text extraction
            - Test multi-line paragraph extraction
            - Verify text accuracy (>95% for clean images)
        - **Language support**:
            - Test English text
            - Test multiple languages (if language packs installed)
            - Test language auto-detection
        - **Image quality handling**:
            - Test high-quality scans
            - Test low-quality scans
            - Test rotated images
            - Test skewed images
        - **Configuration options**:
            - Test different PSM (Page Segmentation Modes)
            - Test OCR confidence scores
            - Test bounding box extraction
        - **Error handling**:
            - Test with invalid image data
            - Test with unsupported formats
            - Test with corrupted images
            - Test timeout handling
    - **Acceptance**: 15+ comprehensive Tesseract tests

1. **Python OCR Backend Integration Tests (1.5 hours)**

    - **File**: `crates/kreuzberg/tests/python_ocr_integration_tests.rs` (new)
    - **Prerequisites**:
        - Python interpreter available
        - PyO3 bindings built
        - Mock Python OCR backend for testing
    - **Tests**:
        - **FFI Bridge Tests**:
            - Test Python backend registration from Rust
            - Test calling Python backend from Rust
            - Test GIL management (no deadlocks)
            - Test async execution via spawn_blocking
            - Test error propagation (Python exceptions → Rust)
        - **Mock Backend Tests**:
            - Create mock Python OCR backend in test
            - Register with Rust core
            - Extract with Rust API using Python backend
            - Verify results passed correctly
        - **Real Backend Tests** (if EasyOCR/PaddleOCR available):
            - Test EasyOCR via FFI bridge
            - Test PaddleOCR via FFI bridge
            - Compare results with Tesseract
            - Test language selection
        - **Concurrent Tests**:
            - Register multiple Python backends
            - Test parallel extraction calls
            - Verify no GIL contention issues
    - **Acceptance**: 12+ Python FFI integration tests

1. **PDF OCR Integration Tests (1 hour)**

    - **File**: `crates/kreuzberg/tests/pdf_ocr_integration_tests.rs` (new)
    - **Test Scenarios**:
        - **Text PDFs**:
            - Extract from native text PDF (no OCR needed)
            - Verify OCR skipped when text available
        - **Image PDFs**:
            - Extract from scanned PDF (OCR required)
            - Test multi-page scanned PDFs
            - Verify all pages processed
        - **Mixed PDFs**:
            - Extract from PDF with both text and images
            - Verify text extracted directly, images via OCR
        - **Configuration**:
            - Test `force_ocr` flag
            - Test OCR backend selection
            - Test language configuration
        - **Performance**:
            - Test batch processing of PDF pages
            - Measure OCR overhead
    - **Acceptance**: 10+ PDF OCR integration tests

1. **Image OCR Integration Tests (45 min)**

    - **File**: `crates/kreuzberg/tests/image_ocr_integration_tests.rs` (new)
    - **Test Formats**:
        - PNG images
        - JPEG images
        - TIFF images
        - WebP images (if supported)
    - **Test Scenarios**:
        - Extract from single images
        - Extract from image collections
        - Test different image sizes (small, large, huge)
        - Test different DPI settings
        - Test color vs grayscale images
    - **Acceptance**: 8+ image OCR tests

1. **OCR Performance and Benchmarks (1 hour)**

    - **File**: `crates/kreuzberg/benches/ocr_benchmarks.rs` (new)
    - **Benchmarks**:
        - Tesseract extraction speed (various image sizes)
        - Python OCR backend overhead (FFI + GIL)
        - Multi-page PDF OCR throughput
        - Concurrent OCR requests
        - Memory usage during OCR
    - **Metrics**:
        - Time per page
        - Throughput (pages/second)
        - Memory consumption
        - GIL contention impact
    - **Acceptance**: Performance baseline established

1. **OCR Accuracy Testing (30 min)**

    - **File**: `crates/kreuzberg/tests/ocr_accuracy_tests.rs` (new)
    - **Setup**:
        - Ground truth text files
        - Corresponding test images
        - Character-level accuracy calculation
    - **Tests**:
        - Measure Tesseract accuracy on clean text
        - Measure accuracy on degraded images
        - Compare different backends
        - Test edge cases (very small text, unusual fonts)
    - **Acceptance**: Accuracy metrics tracked

1. **End-to-End OCR Workflows (45 min)**

    - **File**: `crates/kreuzberg/tests/e2e_ocr_workflows.rs` (new)
    - **Workflows**:
        - **Invoice Processing**:
            - Extract scanned invoice
            - Verify key fields extracted
            - Test with multiple invoice formats
        - **Document Digitization**:
            - Process multi-page scanned document
            - Verify page order preserved
            - Test with different document types
        - **Table Extraction**:
            - Extract tables from images
            - Verify table structure preserved
            - Test with complex tables
    - **Acceptance**: 5+ real-world workflow tests

**Completion Criteria**:

- ✅ 50+ new Rust integration tests
- ✅ All OCR backends tested (Tesseract + Python FFI)
- ✅ Real image processing tested extensively
- ✅ Performance benchmarks established
- ✅ Accuracy metrics tracked
- ✅ End-to-end workflows validated
- ✅ Documentation of test coverage
- ✅ CI/CD integration (tests run on every commit)

**Estimated Time**: 4-6 hours

______________________________________________________________________

## 📊 Overall Progress Tracking

**Estimated Total Time**: 11-16 hours

- [ ] **Task 1**: Fix PostProcessor Tests (30-60 min)
- [ ] **Task 2**: Extraction Pipeline Integration (1-2 hours)
- [ ] **Task 3**: OCR Backend Migration (3-4 hours)
- [ ] **Task 4**: MCP/API CLI Commands (2-3 hours)
- [ ] **Task 5**: Rust Integration Tests + OCR (4-6 hours)

**Current Completion**: ~85%
**Target**: 100% (production-ready v4.0)

______________________________________________________________________

## 🎯 Success Criteria for v4.0 Release

- ✅ Rust core: 839 tests passing
- ✅ Python PostProcessors: Implemented and tested
- ✅ Python OCR FFI: Bridge complete
- 🔲 Python tests: 71/71 passing (currently 56/71)
- 🔲 OCR backends: Migrated and working
- 🔲 Integration tests: 50+ Rust tests with real OCR
- 🔲 CLI commands: MCP and API exposed
- 🔲 Documentation: Complete and accurate
- 🔲 Coverage: ≥95% (currently ~92-94%)

______________________________________________________________________

## 📝 Notes

### Build Configuration

**Python Bindings**:

```bash
cd packages/python
maturin develop --release
```

**Rust CLI**:

```bash
# Basic build
cargo build --release

# With MCP
cargo build --release --features mcp

# With API
cargo build --release --features api

# With all features
cargo build --release --all-features
```

### Testing

**Python Tests**:

```bash
cd packages/python
uv run pytest tests/postprocessors/ -v
```

**Rust Tests**:

```bash
cd crates/kreuzberg
cargo test --release
cargo test --release --features ocr  # Include OCR tests
```

**Integration Tests**:

```bash
cargo test --release --test '*' --all-features
```

### Dependencies for Testing

**OCR Testing Requirements**:

- Tesseract: `brew install tesseract` (macOS) or `apt-get install tesseract-ocr` (Linux)
- Language packs: `brew install tesseract-lang` (optional, for multilingual tests)
- Test images: Located in `crates/kreuzberg/tests/fixtures/images/`

**Python OCR Backends** (optional):

- EasyOCR: `pip install easyocr`
- PaddleOCR: `pip install paddleocr paddlepaddle`

______________________________________________________________________

## 🔧 Troubleshooting

### Common Issues

1. **Bindings not updating after Rust changes**

    - Solution: Run `maturin develop --release` from `packages/python/`

1. **Tests can't find \_internal_bindings**

    - Solution: Ensure `module-name = "kreuzberg._internal_bindings"` in pyproject.toml
    - Rebuild with maturin from correct directory

1. **OCR tests failing**

    - Check Tesseract installed: `which tesseract`
    - Check language packs: `tesseract --list-langs`
    - Verify test images exist in fixtures directory

1. **Python tests failing with spaCy model errors**

    - Download model: `python -m spacy download en_core_web_sm`
    - Or skip tests: Tests should auto-skip if model missing

______________________________________________________________________

**Next Action**: Start with Task 1 (Fix PostProcessor Tests)
