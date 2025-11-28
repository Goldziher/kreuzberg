# C# Binding Parity TODO

## Executive Summary

The C# bindings (on `csharp-bindings` branch) are **85% functionally complete** but have significant gaps in:
1. **API Surface**: Missing embedding support (3 functions + 2 types)
2. **CI/CD Infrastructure**: Single-stage CI instead of build→test pattern
3. **Documentation**: Only 40% parity with other languages (8/65 examples, no API reference)

**Target**: Achieve 100% parity with Python/TypeScript/Java/Go/Ruby bindings before merging to main.

---

## PRIORITY 1: CRITICAL - API Surface Gaps

### 1.1 Add Embedding Support
**Impact**: High - Critical for RAG/semantic search workflows
**Files**:
- `packages/csharp/Kreuzberg/Models.cs`
- `packages/csharp/Kreuzberg/KreuzbergClient.cs`
- `packages/csharp/Kreuzberg/NativeMethods.cs`

**Tasks**:
- [ ] Add `EmbeddingConfig` class to Models.cs
  - Properties: `Provider`, `Model`, `ApiKey`, `Endpoint`, `BatchSize`, `Dimensions`
- [ ] Add `EmbeddingPreset` class to Models.cs
  - Properties: `Name`, `Description`, `ModelName`, `ChunkSize`, `ChunkOverlap`, `Dimensions`, `ContextLength`
- [ ] Add P/Invoke declarations to NativeMethods.cs:
  ```csharp
  [DllImport("kreuzberg_ffi")]
  internal static extern IntPtr kreuzberg_list_embedding_presets();

  [DllImport("kreuzberg_ffi")]
  internal static extern IntPtr kreuzberg_get_embedding_preset(IntPtr name);
  ```
- [ ] Add public API methods to KreuzbergClient.cs:
  ```csharp
  public static List<EmbeddingPreset> ListEmbeddingPresets()
  public static EmbeddingPreset GetEmbeddingPreset(string name)
  ```
- [ ] Add XML documentation to all new types and methods
- [ ] Write unit tests in `Kreuzberg.Tests/EmbeddingTests.cs`

**Reference**: Python `kreuzberg/__init__.py` lines for embedding functions, TypeScript `packages/typescript/src/index.ts`

---

### 1.2 Add HTML Processing Options
**Impact**: Medium - Affects HTML document processing quality
**Files**: `packages/csharp/Kreuzberg/Models.cs`

**Tasks**:
- [ ] Add `HtmlConversionOptions` class:
  - Properties: `IncludeImages`, `IncludeLinks`, `PreserveFormatting`, `ExtractMetadata`
- [ ] Add `HtmlPreprocessingOptions` class:
  - Properties: `RemoveScripts`, `RemoveStyles`, `RemoveComments`, `NormalizeWhitespace`
- [ ] Update `ExtractionConfig` to include `HtmlConversionOptions` property
- [ ] Add XML documentation
- [ ] Write tests

**Reference**: Python `kreuzberg/types.py`, Java config classes

---

### 1.3 Add MIME Validation API (Optional)
**Impact**: Low - Minor utility function (TypeScript-only feature)
**Files**: `packages/csharp/Kreuzberg/KreuzbergClient.cs`, `NativeMethods.cs`

**Tasks**:
- [ ] Add P/Invoke for `kreuzberg_validate_mime_type`
- [ ] Add `public static bool ValidateMimeType(string mimeType)` to KreuzbergClient
- [ ] Add XML documentation
- [ ] Write tests

---

## PRIORITY 2: CRITICAL - CI/CD Infrastructure

### 2.1 Split CI Workflow into Build→Test Pattern
**Impact**: High - Ensures pre-built binaries are tested, not rebuilt
**Files**: `.github/workflows/ci-csharp.yaml`

**Tasks**:
- [ ] Rename existing `csharp` job to `build-csharp` (lines 24-75)
- [ ] In `build-csharp` job:
  - [ ] Add `dotnet build --configuration Release` step
  - [ ] Add `dotnet pack --configuration Release --no-build` step
  - [ ] Add `actions/upload-artifact@v6` step to upload NuGet package
  - [ ] Remove test execution from this job
- [ ] Create new `test-csharp` job:
  - [ ] Add `needs: [build-csharp]` dependency
  - [ ] Add `actions/download-artifact@v6` to download package
  - [ ] Add `dotnet test` using downloaded artifact
  - [ ] Keep existing matrix strategy (ubuntu/macos/windows)
- [ ] Verify library path setup (LD_LIBRARY_PATH, DYLD_LIBRARY_PATH)

**Reference Pattern**:
- Python: `.github/workflows/ci-python.yaml` lines 24-123 (build-and-smoke-python) + 125-192 (test-python)
- Java: `.github/workflows/ci-java.yaml` lines 24-78 (build-java) + 80-143 (test-java)

---

### 2.2 Add Smoke Test Action
**Impact**: Important - Verifies package integrity post-build
**Files**:
- `.github/actions/smoke-csharp/action.yml` (NEW)
- `.github/workflows/ci-csharp.yaml` (update)

**Tasks**:
- [ ] Create `.github/actions/smoke-csharp/action.yml`:
  ```yaml
  name: 'Smoke Test C#'
  description: 'Verify C# bindings can be imported and used'
  runs:
    using: composite
    steps:
      - name: Test package import
        shell: bash
        run: |
          dotnet new console -o smoke-test
          cd smoke-test
          dotnet add package Kreuzberg
          dotnet run
  ```
- [ ] Add smoke test step to `build-csharp` job after artifact upload
- [ ] Write simple test program that:
  - Imports Kreuzberg
  - Calls `GetVersion()`
  - Calls `DetectMimeType()` with sample data
  - Exits successfully

**Reference**: `.github/actions/smoke-python/action.yml`, `.github/actions/smoke-node/action.yml`

---

### 2.3 Fix Publishing Workflow
**Impact**: High - Ensures NuGet packages are built in CI, not at publish time
**Files**: `.github/workflows/publish.yaml`

**Tasks**:
- [ ] Add new `csharp-package` job (insert after line 932, before `publish-nuget`):
  ```yaml
  csharp-package:
    name: Build C# Package
    needs: [prepare]
    if: needs.prepare.outputs.release_csharp == 'true'
    runs-on: ubuntu-latest
    steps:
      - checkout
      - build kreuzberg-ffi
      - dotnet restore
      - dotnet build --configuration Release
      - dotnet pack --configuration Release --no-build
      - upload-artifact (nuget package)
  ```
- [ ] Update `upload-release-artifacts` job (line 1168-1179):
  - [ ] Add `csharp-package` to `needs:` array
- [ ] Update `publish-nuget` job (lines 1691-1735):
  - [ ] Add `download-artifact` step before pack
  - [ ] Remove `dotnet restore` and `dotnet build` steps
  - [ ] Use pre-built artifact for `dotnet nuget push`

**Reference**: Java package job at lines 933-992, Python wheels at lines 321-450

---

## PRIORITY 3: HIGH - Documentation Gaps

### 3.1 Create API Reference Documentation
**Impact**: High - Users need comprehensive API docs
**Files**: `docs/reference/api-csharp.md` (NEW)

**Tasks**:
- [ ] Create `docs/reference/api-csharp.md` (target: 400-500 lines)
- [ ] Document all public API methods from KreuzbergClient.cs:
  - Extraction APIs (8 methods)
  - Configuration APIs (2 methods)
  - MIME Type APIs (3 methods)
  - Plugin Registry APIs (15 methods)
  - Embedding APIs (2 methods - after implementing)
  - Utility APIs (1 method)
- [ ] Document all public types from Models.cs:
  - ExtractionResult, Metadata, Table, Chunk, ExtractedImage
  - All config classes (17+ types)
  - Format-specific metadata (10+ types)
- [ ] Document exception hierarchy from Errors.cs
- [ ] Add code examples for each API method
- [ ] Add cross-references to guides

**Reference Structure**: `docs/reference/api-java.md` (489 lines), `docs/reference/api-python.md`

---

### 3.2 Add Installation Guide
**Impact**: High - Blocks user onboarding
**Files**:
- `docs/getting-started/installation.md` (update)
- `README.md` (update root)

**Tasks**:
- [ ] Add C# section to `docs/getting-started/installation.md` (after Ruby, before Rust):
  ```markdown
  ## C#

  Install via NuGet Package Manager:

  ```bash
  dotnet add package Kreuzberg
  ```

  Or via Package Manager Console:
  ```
  Install-Package Kreuzberg
  ```

  ### System Dependencies
  - .NET 9.0 or later
  - Native dependencies (see main installation guide)
  ```
- [ ] Update root `README.md`:
  - [ ] Add C# to feature list (line 20): "Rust, Python, Ruby, TypeScript/Node.js, **C#/.NET**, Java, Go"
  - [ ] Add C# installation section (lines 57-117, insert after Go)
  - [ ] Add C# to Quick Start section (lines 119-129)

---

### 3.3 Create Code Snippet Examples
**Impact**: High - Users need practical examples
**Files**: `docs/snippets/csharp/` (57 NEW files)

**Tasks**:

#### Configuration Examples (5 files):
- [ ] `advanced_config.md` - Comprehensive configuration with all options
- [ ] `config_basic.md` - Basic ExtractionConfig instantiation
- [ ] `config_programmatic.md` - Building config in code
- [ ] `config_discover.md` - Using DiscoverExtractionConfig()
- [ ] `config_file.md` - Loading config from TOML/YAML/JSON

#### Plugin System Examples (13 files):
- [ ] `plugin_extractor.md` - Custom document extractor
- [ ] `plugin_validator.md` - Custom validator implementation
- [ ] `plugin_testing.md` - Testing custom plugins
- [ ] `plugin_logging.md` - Plugin with logging
- [ ] `extractor_registration.md` - RegisterDocumentExtractor usage
- [ ] `list_plugins.md` - Listing all registered plugins
- [ ] `unregister_plugins.md` - Removing plugins
- [ ] `clear_plugins.md` - Clearing all plugins
- [ ] `quality_score_validator.md` - Quality validation example
- [ ] `min_length_validator.md` - Length validation
- [ ] `word_count_processor.md` - Text processing postprocessor
- [ ] `pdf_only_processor.md` - Format-specific processing
- [ ] `pdf_metadata_extractor.md` - PDF metadata extraction

#### Advanced Features (20 files):
- [ ] `chunking.md` - Basic text chunking
- [ ] `chunking_config.md` - ChunkingConfig setup
- [ ] `chunking_rag.md` - RAG-optimized chunking
- [ ] `language_detection.md` - Basic language detection
- [ ] `language_detection_config.md` - LanguageDetectionConfig
- [ ] `language_detection_multilingual.md` - Multi-language support
- [ ] `token_reduction.md` - Basic token reduction
- [ ] `token_reduction_config.md` - TokenReductionConfig
- [ ] `token_reduction_example.md` - Real-world token reduction
- [ ] `image_extraction.md` - Extracting images from documents
- [ ] `image_preprocessing.md` - Image preprocessing options
- [ ] `embedding_config.md` - EmbeddingConfig setup
- [ ] `embedding_with_chunking.md` - Embeddings + chunking
- [ ] `keyword_extraction_config.md` - Keyword extraction setup
- [ ] `keyword_extraction_example.md` - Keyword usage
- [ ] `metadata.md` - Metadata extraction patterns
- [ ] `tables.md` - Table extraction
- [ ] `vector_database_integration.md` - Vector DB integration
- [ ] `combining_all_features.md` - Comprehensive example
- [ ] `complete_example.md` - End-to-end pipeline

#### Integration Examples (10 files):
- [ ] `client_extract_single_file.md` - Basic client usage
- [ ] `client_extract_multiple_files.md` - Batch processing
- [ ] `client_extract_with_config.md` - Client with configuration
- [ ] `postprocessor_config.md` - Postprocessor configuration
- [ ] `quality_processing_config.md` - Quality processing
- [ ] `quality_processing_example.md` - Quality processing patterns
- [ ] `pdf_config.md` - PDF-specific configuration
- [ ] `html_conversion_options.md` - HTML conversion config
- [ ] `mcp_langchain_integration.md` - LangChain integration
- [ ] `mcp_server_start.md` - MCP server setup

#### OCR Examples (9 files):
- [ ] `tesseract_config.md` - Tesseract OCR setup
- [ ] `ocr_dpi_config.md` - DPI configuration
- [ ] `ocr_force_all_pages.md` - Force OCR on all pages
- [ ] `ocr_multi_language.md` - Multi-language OCR
- [ ] `ocr_preprocessing.md` - Image preprocessing for OCR
- [ ] `ocr_backend_selection.md` - Choosing OCR backend
- [ ] `ocr_quality_settings.md` - Quality vs speed tradeoffs
- [ ] `ocr_custom_backend.md` - Custom OCR backend
- [ ] `cloud_ocr_backend.md` - Cloud OCR integration

**Reference**: `docs/snippets/python/` (65 files), `docs/snippets/java/` (63 files)

---

### 3.4 Integrate C# into Guides
**Impact**: High - Users need guidance on advanced features
**Files**: 4 guide files to update

**Tasks**:

#### OCR Guide:
- [ ] Add C# tab to `docs/guides/ocr.md`
- [ ] Include: Tesseract config, multi-language setup, preprocessing
- [ ] Add 3-5 C# code examples

#### Configuration Guide:
- [ ] Add C# tab to `docs/guides/configuration.md`
- [ ] Include: File loading, programmatic config, discovery
- [ ] Add 4-6 C# code examples

#### Plugins Guide:
- [ ] Add C# tab to `docs/guides/plugins.md`
- [ ] Include: PostProcessor, Validator, OcrBackend implementations
- [ ] Add 5-7 C# code examples

#### Advanced Guide:
- [ ] Add C# tab to `docs/guides/advanced.md`
- [ ] Include: Chunking, embeddings, language detection, token reduction
- [ ] Add 6-8 C# code examples

**Reference**: Each guide already has tabs for Python, TypeScript, Ruby, Java, Go - add C# as 6th tab

---

### 3.5 Enhance XML Documentation Comments
**Impact**: Medium - Improves IDE IntelliSense
**Files**: All C# source files in `packages/csharp/Kreuzberg/`

**Tasks**:

#### Models.cs (861 lines, 0 XML comments):
- [ ] Add XML docs to all 17+ config classes
- [ ] Add XML docs to ExtractionResult, Metadata, Table, Chunk
- [ ] Add XML docs to all format-specific metadata classes (10+ types)
- [ ] Include `<summary>`, `<param>`, `<returns>`, `<example>` tags
- [ ] Target: 200+ XML comment lines

#### Errors.cs (233 lines, 0 XML comments):
- [ ] Add XML docs to all 11 exception classes
- [ ] Document when each exception is thrown
- [ ] Add usage examples
- [ ] Target: 50+ XML comment lines

#### NativeMethods.cs (0 XML comments):
- [ ] Add XML docs to P/Invoke declarations
- [ ] Document native function contracts
- [ ] Add safety notes for IntPtr usage
- [ ] Target: 40+ XML comment lines

#### InteropUtilities.cs (0 XML comments):
- [ ] Add XML docs to all utility methods
- [ ] Document memory management patterns
- [ ] Add safety notes
- [ ] Target: 30+ XML comment lines

#### Serialization.cs (0 XML comments):
- [ ] Add XML docs to serialization methods
- [ ] Document JSON handling
- [ ] Add examples
- [ ] Target: 20+ XML comment lines

#### KreuzbergClient.cs (59 XML comment lines):
- [ ] Complete missing documentation for ~10 undocumented methods
- [ ] Add `<exception>` tags to all methods
- [ ] Add `<example>` tags to complex methods
- [ ] Target: 120+ XML comment lines (currently 59)

**Total Target**: 460+ new XML documentation lines

---

### 3.6 Expand Package README
**Impact**: Medium - Improves first-time user experience
**Files**: `packages/csharp/README.md`

**Tasks**:
- [ ] Add feature highlights section (what Kreuzberg does)
- [ ] Add requirements section (.NET 9.0+, system dependencies)
- [ ] Add system dependencies section
- [ ] Add troubleshooting section (common errors, native library issues)
- [ ] Add comprehensive configuration examples
- [ ] Add OCR backend selection guidance
- [ ] Add table extraction examples
- [ ] Add error handling patterns
- [ ] Add advanced features overview (chunking, language detection, etc.)
- [ ] Add links to full documentation site
- [ ] Add license section
- [ ] Add badges (NuGet version, CI status, license)
- [ ] Target: Expand from 48 lines to 150+ lines

**Reference**: `packages/python/README.md` (455 lines), `packages/typescript/README.md` (439 lines)

---

## PRIORITY 4: MEDIUM - Testing & Quality

### 4.1 Add Embedding Preset Tests
**Impact**: Medium - Ensures new API works correctly
**Files**: `packages/csharp/Kreuzberg.Tests/EmbeddingTests.cs` (NEW)

**Tasks**:
- [ ] Create test file with xUnit tests
- [ ] Test `ListEmbeddingPresets()` returns non-empty list
- [ ] Test `GetEmbeddingPreset("default")` returns valid preset
- [ ] Test `GetEmbeddingPreset("invalid")` throws exception
- [ ] Test embedding config serialization/deserialization
- [ ] Test integration with ExtractionConfig

---

### 4.2 Add HTML Options Tests
**Impact**: Medium - Validates new config types
**Files**: `packages/csharp/Kreuzberg.Tests/ConfigTests.cs` (NEW or extend existing)

**Tasks**:
- [ ] Test HtmlConversionOptions instantiation
- [ ] Test HtmlPreprocessingOptions instantiation
- [ ] Test HTML options in ExtractionConfig
- [ ] Test config serialization with HTML options

---

### 4.3 Expand E2E Test Coverage
**Impact**: Low - E2E tests are auto-generated, but manual tests valuable
**Files**: `e2e/csharp/ManualTests/` (NEW directory)

**Tasks**:
- [ ] Add manual integration tests for:
  - Plugin registration/unregistration
  - Config discovery
  - Batch extraction error handling
  - Async cancellation
- [ ] Add performance tests for large documents
- [ ] Add memory leak tests for repeated extraction

---

## PRIORITY 5: LOW - Minor Improvements

### 5.1 Add Examples Directory
**Impact**: Low - Nice-to-have for developers
**Files**: `examples/csharp/` (NEW directory)

**Tasks**:
- [ ] Create `examples/csharp/BasicExtraction/` console app
- [ ] Create `examples/csharp/ConfigurationPatterns/` console app
- [ ] Create `examples/csharp/CustomPlugins/` console app
- [ ] Create `examples/csharp/OcrSetup/` console app
- [ ] Create `examples/csharp/ErrorHandling/` console app
- [ ] Create `examples/csharp/RealWorldUseCases/` console app
- [ ] Add README.md for each example

---

### 5.2 Taskfile Cleanup
**Impact**: Low - Minor duplication
**Files**: `Taskfile.yaml`

**Tasks**:
- [ ] Remove duplicate `csharp:format` task (lines 710-716 duplicate 694-700)
- [ ] Remove duplicate `csharp:format:check` task (lines 718-724 duplicate 702-708)
- [ ] Or reorganize to have format → lint instead of duplication

---

### 5.3 Verify Version Sync
**Impact**: Low - Ensure automation works
**Files**: `scripts/sync_versions.py`

**Tasks**:
- [ ] Review `sync_versions.py` to confirm it updates C# `.csproj` files
- [ ] Test `task version:sync` with C# branch
- [ ] Add C# to sync target if missing

---

## Summary Statistics

### API Gaps:
- **Missing Functions**: 3 (ListEmbeddingPresets, GetEmbeddingPreset, ValidateMimeType)
- **Missing Types**: 3 (EmbeddingConfig, EmbeddingPreset, HtmlConversionOptions)

### CI/CD Gaps:
- **Workflows to Split**: 1 (ci-csharp.yaml)
- **New Actions Needed**: 1 (smoke-csharp)
- **Publish Jobs to Fix**: 2 (csharp-package, publish-nuget)

### Documentation Gaps:
- **Missing API Reference**: 1 file (400-500 lines)
- **Missing Installation Guide**: 2 sections (docs + root README)
- **Missing Snippet Files**: 57 files
- **Guides Needing C# Tabs**: 4 files
- **XML Documentation Lines Needed**: 460+ lines
- **Package README Expansion**: 100+ lines

### Testing Gaps:
- **New Test Files Needed**: 3 (EmbeddingTests, ConfigTests, ManualTests)

### Total Tasks: 148 concrete action items

---

## Completion Checklist

Once all tasks are complete, verify:
- [ ] All API methods from other bindings are implemented in C#
- [ ] CI workflow follows build→test pattern
- [ ] Smoke tests pass on all platforms
- [ ] Publishing workflow uses pre-built artifacts
- [ ] API reference documentation exists and is comprehensive
- [ ] C# appears in all relevant guides with code examples
- [ ] 57+ snippet examples exist covering all major features
- [ ] XML documentation on all public APIs
- [ ] Package README is comprehensive
- [ ] All tests pass
- [ ] `task e2e:csharp:verify` succeeds
- [ ] `task csharp:lint:check` succeeds
- [ ] C# mentioned in root README alongside other languages

**Estimated Effort**: 2-3 weeks for full parity (8-10 developer days)

---

## Notes

- The C# binding is already 85% complete - this TODO focuses on closing the final 15% gap
- Most gaps are documentation and CI/CD infrastructure, not core functionality
- The FFI bridge and plugin system are well-designed and functional
- Once merged to main, C# will have full parity with other language bindings
