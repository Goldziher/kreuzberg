# Kreuzberg Documentation & Package Metadata TODO

Generated: 2025-11-28

This document consolidates all identified gaps from comprehensive audits of package manifests, documentation, and language parity.

---

## 🔴 Critical Priority

### C# Language Support

**Status**: Severely underdeveloped (11.9% snippet coverage vs 92%+ for other languages)

#### Missing C# Snippets (57 total)

**Core Configuration (13 files)**
- [ ] `docs/snippets/csharp/config/basic.cs`
- [ ] `docs/snippets/csharp/config/custom_mime_types.cs`
- [ ] `docs/snippets/csharp/config/disable_cache.cs`
- [ ] `docs/snippets/csharp/config/enable_cache.cs`
- [ ] `docs/snippets/csharp/config/full_example.cs`
- [ ] `docs/snippets/csharp/config/include_meta.cs`
- [ ] `docs/snippets/csharp/config/ocr_lang.cs`
- [ ] `docs/snippets/csharp/config/parse_links.cs`
- [ ] `docs/snippets/csharp/config/parse_metadata.cs`
- [ ] `docs/snippets/csharp/config/postprocessor.cs`
- [ ] `docs/snippets/csharp/config/validator.cs`
- [ ] `docs/snippets/csharp/config/with_cache.cs`
- [ ] `docs/snippets/csharp/config/with_timeout.cs`

**Advanced Features (13 files)**
- [ ] `docs/snippets/csharp/advanced/async_extraction.cs`
- [ ] `docs/snippets/csharp/advanced/batch_processing.cs`
- [ ] `docs/snippets/csharp/advanced/custom_cache.cs`
- [ ] `docs/snippets/csharp/advanced/custom_extractor.cs`
- [ ] `docs/snippets/csharp/advanced/custom_ocr_backend.cs`
- [ ] `docs/snippets/csharp/advanced/custom_postprocessor.cs`
- [ ] `docs/snippets/csharp/advanced/custom_validator.cs`
- [ ] `docs/snippets/csharp/advanced/error_handling.cs`
- [ ] `docs/snippets/csharp/advanced/extract_from_bytes.cs`
- [ ] `docs/snippets/csharp/advanced/extract_from_url.cs`
- [ ] `docs/snippets/csharp/advanced/extract_with_config.cs`
- [ ] `docs/snippets/csharp/advanced/plugin_registry.cs`
- [ ] `docs/snippets/csharp/advanced/streaming.cs`

**OCR & Metadata (8 files)**
- [ ] `docs/snippets/csharp/ocr/auto_ocr.cs`
- [ ] `docs/snippets/csharp/ocr/easyocr_backend.cs`
- [ ] `docs/snippets/csharp/ocr/force_ocr.cs`
- [ ] `docs/snippets/csharp/ocr/tesseract_backend.cs`
- [ ] `docs/snippets/csharp/metadata/author.cs`
- [ ] `docs/snippets/csharp/metadata/created_date.cs`
- [ ] `docs/snippets/csharp/metadata/parse_all.cs`
- [ ] `docs/snippets/csharp/metadata/title.cs`

**Getting Started (6 files)**
- [ ] `docs/snippets/csharp/getting-started/basic_usage.cs`
- [ ] `docs/snippets/csharp/getting-started/extract_file.cs`
- [ ] `docs/snippets/csharp/getting-started/extract_with_ocr.cs`
- [ ] `docs/snippets/csharp/getting-started/hello_world.cs`
- [ ] `docs/snippets/csharp/getting-started/install_verify.cs`
- [ ] `docs/snippets/csharp/getting-started/read_content.cs`

**Plugin System (5 files)**
- [ ] `docs/snippets/csharp/plugins/custom_cache_plugin.cs`
- [ ] `docs/snippets/csharp/plugins/custom_extractor_plugin.cs`
- [ ] `docs/snippets/csharp/plugins/custom_ocr_plugin.cs`
- [ ] `docs/snippets/csharp/plugins/custom_postprocessor_plugin.cs`
- [ ] `docs/snippets/csharp/plugins/custom_validator_plugin.cs`

**API Types (3 files)**
- [ ] `docs/snippets/csharp/api/extraction_config.cs`
- [ ] `docs/snippets/csharp/api/extraction_result.cs`
- [ ] `docs/snippets/csharp/api/kreuzberg.cs`

**MCP Integration (2 files)**
- [ ] `docs/snippets/csharp/mcp/client.cs`
- [ ] `docs/snippets/csharp/mcp/server.cs`

**CLI Usage (2 files)**
- [ ] `docs/snippets/csharp/cli/basic_cli.cs`
- [ ] `docs/snippets/csharp/cli/cli_with_config.cs`

**Utilities (2 files)**
- [ ] `docs/snippets/csharp/utils/detect_language.cs`
- [ ] `docs/snippets/csharp/utils/extract_keywords.cs`

**Caching (1 file)**
- [ ] `docs/snippets/csharp/cache/disk_cache.cs`

**Docker (1 file)**
- [ ] `docs/snippets/csharp/docker/usage.cs`

**Benchmarking (1 file)**
- [ ] `docs/snippets/csharp/benchmarking/simple_benchmark.cs`

### Rust Crate Metadata

**kreuzberg-cli (crates/kreuzberg-cli)**
- [ ] Create comprehensive `README.md` with installation, usage examples, CLI options
- [ ] Verify all Cargo.toml fields are complete

**kreuzberg-ffi (crates/kreuzberg-ffi)**
- [ ] Create comprehensive `README.md` with C API usage, FFI safety guidelines
- [ ] Verify all Cargo.toml fields are complete

### Documentation Critical Issues

**Installation & Setup (docs/guides/installation.md)**
- [ ] Add Java installation instructions with Maven dependencies
- [ ] Add Go installation instructions with go get command
- [ ] Add C# NuGet installation instructions (may already exist, verify completeness)

**API Documentation (docs/api/types.md)**
- [ ] Document `chunks` property on ExtractionResult with type definitions
- [ ] Document `images` property on ExtractionResult with type definitions
- [ ] Add usage examples for chunk-based processing

**Feature Comparison (docs/index.md or docs/guides/overview.md)**
- [ ] Update feature comparison table to include Java column
- [ ] Update feature comparison table to include Go column
- [ ] Verify C# feature support is accurately documented

---

## 🟠 High Priority

### TypeScript Snippet Gaps (13 files)

- [ ] `docs/snippets/typescript/advanced/custom_cache.ts`
- [ ] `docs/snippets/typescript/advanced/custom_extractor.ts`
- [ ] `docs/snippets/typescript/advanced/custom_ocr_backend.ts`
- [ ] `docs/snippets/typescript/advanced/custom_postprocessor.ts`
- [ ] `docs/snippets/typescript/advanced/custom_validator.ts`
- [ ] `docs/snippets/typescript/advanced/plugin_registry.ts`
- [ ] `docs/snippets/typescript/plugins/custom_cache_plugin.ts`
- [ ] `docs/snippets/typescript/plugins/custom_extractor_plugin.ts`
- [ ] `docs/snippets/typescript/plugins/custom_ocr_plugin.ts`
- [ ] `docs/snippets/typescript/plugins/custom_postprocessor_plugin.ts`
- [ ] `docs/snippets/typescript/plugins/custom_validator_plugin.ts`
- [ ] `docs/snippets/typescript/mcp/client.ts`
- [ ] `docs/snippets/typescript/mcp/server.ts`

### Go Package Documentation

**Critical: Missing pkg.go.dev Documentation**
- [ ] Create `packages/go/kreuzberg/doc.go` with comprehensive package-level documentation
- [ ] Include overview, installation, basic usage examples
- [ ] Add links to documentation site and repository
- [ ] Ensure examples are runnable and tested

### Java Maven Central Publishing

**pom.xml Configuration**
- [ ] Add `<distributionManagement>` section with ossrh repository
- [ ] Add `maven-javadoc-plugin` for javadoc artifact generation
- [ ] Add `maven-source-plugin` for source artifact generation
- [ ] Add `maven-gpg-plugin` for artifact signing
- [ ] Add `<scm>` section with Git connection details
- [ ] Verify `<developers>` section is complete

### C# Package Metadata Enhancement

**Kreuzberg.csproj**
- [ ] Expand `<Description>` to 2-3 sentences with key features
- [ ] Add more `<PackageTags>` (ocr, pdf, docx, excel, document-processing, ffi, rust-bindings)
- [ ] Verify `<PackageIcon>` is included if icon file exists
- [ ] Add `<PackageReleaseNotes>` field for version changelog

### Documentation High Priority Issues

**Guides - Configuration (docs/guides/configuration.md)**
- [ ] Add complete C# examples to all configuration sections
- [ ] Verify TypeScript examples are complete and accurate
- [ ] Add cross-language comparison table for config options

**Guides - OCR (docs/guides/ocr.md)**
- [ ] Add complete C# examples for all OCR backends
- [ ] Document EasyOCR Python-specific implementation
- [ ] Document PaddleOCR Python-specific implementation
- [ ] Add performance comparison table for OCR backends

**Guides - Advanced (docs/guides/advanced.md)**
- [ ] Add complete C# examples for custom plugins
- [ ] Add complete TypeScript examples for custom plugins
- [ ] Document plugin lifecycle and thread safety requirements

**MCP Integration (docs/guides/mcp.md or similar)**
- [ ] Add C# MCP client/server examples
- [ ] Add TypeScript MCP client/server examples
- [ ] Add Go MCP client/server examples
- [ ] Add Java MCP client/server examples

---

## 🟡 Medium Priority

### Python Snippet Gaps (2 files)

- [ ] `docs/snippets/python/mcp/client.py`
- [ ] `docs/snippets/python/mcp/server.py`

### Go Snippet Gaps (3 files)

- [ ] `docs/snippets/go/mcp/client.go`
- [ ] `docs/snippets/go/mcp/server.go`
- [ ] `docs/snippets/go/benchmarking/simple_benchmark.go`

### Ruby Snippet Gaps (4 files)

- [ ] `docs/snippets/ruby/mcp/client.rb`
- [ ] `docs/snippets/ruby/mcp/server.rb`
- [ ] `docs/snippets/ruby/benchmarking/simple_benchmark.rb`
- [ ] `docs/snippets/ruby/docker/usage.rb`

### Java Snippet Gaps (4 files)

- [ ] `docs/snippets/java/mcp/client.java`
- [ ] `docs/snippets/java/mcp/server.java`
- [ ] `docs/snippets/java/benchmarking/simple_benchmark.java`
- [ ] `docs/snippets/java/docker/usage.java`

### Rust Snippet Gaps (5 files)

- [ ] `docs/snippets/rust/mcp/client.rs`
- [ ] `docs/snippets/rust/mcp/server.rs`
- [ ] `docs/snippets/rust/benchmarking/simple_benchmark.rs`
- [ ] `docs/snippets/rust/cli/basic_cli.rs`
- [ ] `docs/snippets/rust/cli/cli_with_config.rs`

### Rust Crate Keywords & Categories

**All publishable crates need review:**
- [ ] `crates/kreuzberg/Cargo.toml` - Verify keywords and categories
- [ ] `crates/kreuzberg-py/Cargo.toml` - Add keywords and categories
- [ ] `crates/kreuzberg-node/Cargo.toml` - Add keywords and categories
- [ ] `crates/kreuzberg-rb/Cargo.toml` - Add keywords and categories
- [ ] `crates/kreuzberg-ffi/Cargo.toml` - Add keywords and categories
- [ ] `crates/kreuzberg-cli/Cargo.toml` - Add keywords and categories
- [ ] `crates/kreuzberg-tesseract/Cargo.toml` - Add keywords and categories

### Documentation Medium Priority Issues

**API Documentation**
- [ ] Add comprehensive examples to all API type pages
- [ ] Document all error types with when they occur and how to handle
- [ ] Add migration guide from v3 to v4 if applicable

**Guides - Caching**
- [ ] Ensure all languages have complete cache configuration examples
- [ ] Document cache invalidation strategies
- [ ] Add performance benchmarks for cached vs uncached operations

**Guides - Plugins**
- [ ] Document plugin registration patterns for each language
- [ ] Add thread safety guidelines for plugin implementations
- [ ] Include complete plugin examples for all languages

---

## 🟢 Low Priority

### TypeScript Package Manifest

**package.json Enhancement**
- [ ] Add explicit `readme` field pointing to README.md
- [ ] Add `document-processing` keyword to keyword list

### Documentation Low Priority Issues

**General Documentation Polish**
- [ ] Ensure all code examples have syntax highlighting
- [ ] Verify all internal links are working
- [ ] Add "Edit this page" links to all documentation pages
- [ ] Ensure consistent heading hierarchy across all pages

**Performance Documentation**
- [ ] Add benchmarking guide with all languages
- [ ] Document memory usage characteristics
- [ ] Add performance tuning recommendations

**Deployment Documentation**
- [ ] Add Docker deployment examples for all languages
- [ ] Add Kubernetes deployment examples
- [ ] Add AWS Lambda deployment examples (Python, Node.js)

**FAQ Section**
- [ ] Common errors and solutions
- [ ] Platform-specific issues (Windows, macOS, Linux)
- [ ] Performance troubleshooting

---

## 📊 Summary Statistics

### Language Parity Coverage

- **Python**: 97.0% complete (65/67 snippets)
- **Go**: 95.5% complete (64/67 snippets)
- **Ruby**: 94.0% complete (63/67 snippets)
- **Java**: 94.0% complete (63/67 snippets)
- **Rust**: 92.5% complete (62/67 snippets)
- **TypeScript**: 80.6% complete (54/67 snippets)
- **C#**: 11.9% complete (8/67 snippets) ⚠️

### Documentation Issues by Severity

- **Critical**: 6 issues (installation gaps, API documentation gaps)
- **High**: 14 issues (language-specific examples, MCP integration)
- **Medium**: 43 issues (snippet gaps, metadata enhancements)
- **Low**: 32 issues (polish, additional guides)

### Package Manifest Status

- **Python**: ✅ Excellent (95%) - Production ready
- **TypeScript**: ✅ Excellent (92%) - Minor enhancements recommended
- **Ruby**: ✅ Excellent (95%) - Production ready
- **Rust (core)**: ✅ Ready for publication
- **Rust (cli/ffi)**: ⚠️ Missing READMEs (critical)
- **Java**: ⚠️ Missing Maven Central config (75%)
- **C#**: ⚠️ Needs metadata enhancements (85%)
- **Go**: ⚠️ Missing doc.go (85%)

---

## 🎯 Recommended Implementation Order

1. **Phase 1 - Critical Blockers** (Est. 1-2 weeks)
   - Create Rust crate READMEs for cli and ffi
   - Add Java/Go to installation guide
   - Document ExtractionResult chunks/images properties
   - Create Go doc.go file

2. **Phase 2 - C# Language Parity** (Est. 2-3 weeks)
   - Implement all 57 C# code snippets
   - Update all documentation pages with C# tabs
   - Test all C# examples for correctness

3. **Phase 3 - TypeScript & MCP** (Est. 1 week)
   - Implement 13 missing TypeScript snippets
   - Add MCP client/server examples for all languages

4. **Phase 4 - Package Metadata** (Est. 3-4 days)
   - Configure Java Maven Central publishing
   - Enhance C# package metadata
   - Add keywords/categories to Rust crates

5. **Phase 5 - Documentation Polish** (Est. 1 week)
   - Fill remaining snippet gaps for all languages
   - Add benchmarking examples
   - Create deployment guides
   - Build FAQ section

---

## 📝 Notes

- All audits completed on 2025-11-28
- C# has severe language parity gap requiring immediate attention
- Core functionality is well-documented, gaps are primarily in advanced features
- Python and Ruby packages are production-ready
- Java and Go need publishing infrastructure improvements
- MCP integration documentation is incomplete across all languages

**Audit Sources:**
- Package manifest audit (6 language-specific subagents)
- Documentation audit (1 Sonnet subagent - 95 issues identified)
- Language parity audit (2 subagents - snippet matrix analysis)
