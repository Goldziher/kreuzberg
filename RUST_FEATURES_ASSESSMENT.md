# Rust Crate Feature Flags Assessment

## Current State

All functionality is compiled into the core `kreuzberg` crate by default. This creates a large binary with many dependencies users may not need.

**Current binary size:** ~45-60MB (with all dependencies)

---

## Feature Flag Design

### Core Principles

1. **Minimal default** - Only essential extraction features enabled by default
2. **Format-based** - Users enable formats they need (PDF, Excel, etc.)
3. **Feature-based** - Optional features like OCR, API, MCP are opt-in
4. **Dependency-aligned** - Each feature should map to specific dependencies

---

## Proposed Feature Structure

### Tier 1: Essential (Default)

**Feature: `default`**

```toml
default = ["core-extractors", "mime-detection", "cache"]
```

**What's included:**
- Plain text extraction
- Markdown extraction
- JSON/YAML/TOML parsing
- MIME type detection
- Basic file I/O
- Error handling
- Caching

**Dependencies (minimal):**
- `serde`, `serde_json`, `serde_yaml`, `toml`
- `mime_guess`, `memchr`, `regex`
- `tokio` (runtime)
- `thiserror`, `ahash`

**Binary size:** ~8-12MB

---

### Tier 2: Format Extractors (Opt-in)

#### `feature = "pdf"`

**What it enables:**
- PDF text extraction
- PDF metadata parsing
- PDF image extraction
- PDF rendering

**Dependencies:**
- `pdfium-render` (~30MB with libpdfium)
- `lopdf`

**Use case:** Users working with PDFs

```toml
kreuzberg = { version = "4.0", features = ["pdf"] }
```

#### `feature = "excel"`

**What it enables:**
- Excel (.xlsx, .xls, .ods) extraction
- CSV parsing
- Spreadsheet metadata

**Dependencies:**
- `calamine`
- `polars` (lightweight features only)

**Use case:** Data extraction from spreadsheets

#### `feature = "office"`

**What it enables:**
- PowerPoint (.pptx) extraction
- Word (.docx) extraction via Pandoc
- Legacy Office via LibreOffice conversion

**Dependencies:**
- `roxmltree` (for XML parsing)
- `zip` (for Office Open XML)

**Note:** Requires external tools (Pandoc, LibreOffice)

#### `feature = "email"`

**What it enables:**
- EML parsing
- MSG parsing
- Email metadata extraction

**Dependencies:**
- `mail-parser`
- `msg_parser`

#### `feature = "html"`

**What it enables:**
- HTML to Markdown conversion
- HTML tag stripping
- Inline image handling

**Dependencies:**
- `html-to-markdown-rs` (custom fork)
- `html-escape`

#### `feature = "xml"`

**What it enables:**
- XML streaming parser
- Large XML file support

**Dependencies:**
- `quick-xml`
- `roxmltree`

#### `feature = "archives"`

**What it enables:**
- ZIP extraction
- TAR extraction
- 7z extraction

**Dependencies:**
- `zip`
- `tar`
- `sevenz-rust`

**Use case:** Extracting documents from archives

---

### Tier 3: Processing Features (Opt-in)

#### `feature = "ocr"`

**What it enables:**
- Native Tesseract OCR
- Image preprocessing
- HOCR parsing
- OCR caching

**Dependencies:**
- `tesseract-rs` (requires Tesseract installed)
- `image` (with codecs)
- `fast_image_resize`
- `ndarray`

**Binary impact:** +5MB, requires system Tesseract

**Use case:** OCR-heavy workloads

#### `feature = "language-detection"`

**What it enables:**
- Automatic language detection for extracted text
- Language-based quality scoring
- Multi-language document support

**Dependencies:**
- `whatlang` (~1MB, includes language models)

**Binary impact:** +1-2MB

**Use case:** Multi-language document processing

#### `feature = "chunking"`

**What it enables:**
- Text chunking with overlap
- Semantic chunking
- Markdown-aware chunking

**Dependencies:**
- `text-splitter`

**Binary impact:** +500KB

**Use case:** RAG, LLM preprocessing

#### `feature = "quality"`

**What it enables:**
- Text quality scoring
- Content validation
- Extraction confidence metrics

**Dependencies:**
- `unicode-normalization`
- `chardetng`, `encoding_rs`

**Binary impact:** +1MB

---

### Tier 4: Server Features (Opt-in)

#### `feature = "api"`

**What it enables:**
- REST API server with Axum
- JSON API endpoints
- Health checks
- OpenAPI/Swagger docs

**Dependencies:**
- `axum` (with tokio)
- `tower`, `tower-http`
- `serde_json`

**Binary impact:** +3-4MB

**Use case:** Deploying as HTTP service

**Example:**
```rust
use kreuzberg::api::serve;

#[tokio::main]
async fn main() {
    serve("0.0.0.0:8000").await.unwrap();
}
```

#### `feature = "mcp"`

**What it enables:**
- Model Context Protocol server
- JSON-RPC 2.0 over stdio
- Tool/resource definitions
- Context management

**Dependencies:**
- `jsonrpc-core` or similar
- `serde_json`

**Binary impact:** +1-2MB

**Use case:** Integration with AI tools (Claude Desktop, etc.)

**Example:**
```rust
use kreuzberg::mcp::serve_mcp;

#[tokio::main]
async fn main() {
    serve_mcp().await.unwrap();
}
```

---

### Tier 5: Python Integration (Special)

#### `feature = "python-plugins"`

**What it enables:**
- PyO3-based Python interpreter embedding
- Python plugin registry
- Python OCR backend support (EasyOCR, PaddleOCR)
- Python post-processor support (spaCy, etc.)

**Dependencies:**
- `pyo3` (with auto-initialize)

**Binary impact:** +10-15MB (includes libpython)

**Use case:** Full-featured deployment with Python extensions

**Note:** Only useful for `kreuzberg-cli` and `kreuzberg-api` binaries. The `kreuzberg-py` package always has Python available.

---

## Convenience Feature Bundles

### `feature = "full"`

Everything enabled (current behavior):

```toml
full = [
    "pdf", "excel", "office", "email", "html", "xml", "archives",
    "ocr", "language-detection", "chunking", "quality",
    "api", "mcp"
]
```

**Binary size:** ~50-60MB

### `feature = "server"`

Minimal server deployment:

```toml
server = ["pdf", "excel", "html", "ocr", "api"]
```

**Binary size:** ~35-40MB

### `feature = "cli"`

Interactive CLI usage:

```toml
cli = ["pdf", "excel", "office", "html", "ocr", "language-detection", "chunking"]
```

**Binary size:** ~40-45MB

---

## Updated Cargo.toml Structure

```toml
[package]
name = "kreuzberg"
version = "4.0.0"
edition = "2024"

[features]
default = ["core-extractors", "mime-detection", "cache"]

# Format extractors
pdf = ["pdfium-render", "lopdf"]
excel = ["calamine", "polars"]
office = ["roxmltree", "zip"]
email = ["mail-parser", "msg_parser"]
html = ["html-to-markdown-rs", "html-escape"]
xml = ["quick-xml", "roxmltree"]
archives = ["zip", "tar", "sevenz-rust"]

# Processing features
ocr = ["tesseract-rs", "image", "fast_image_resize", "ndarray"]
language-detection = ["whatlang"]
chunking = ["text-splitter"]
quality = ["unicode-normalization", "chardetng", "encoding_rs"]

# Server features
api = ["axum", "tower", "tower-http"]
mcp = ["jsonrpc-core"]

# Python integration
python-plugins = ["pyo3"]

# Convenience bundles
full = [
    "pdf", "excel", "office", "email", "html", "xml", "archives",
    "ocr", "language-detection", "chunking", "quality",
    "api", "mcp"
]
server = ["pdf", "excel", "html", "ocr", "api"]
cli = ["pdf", "excel", "office", "html", "ocr", "language-detection", "chunking"]

[dependencies]
# Core (always included)
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.48", features = ["rt", "rt-multi-thread", "macros", "fs", "time"] }
thiserror = "2.0"
ahash = "0.8"
async-trait = "0.1"
mime_guess = "2.0"
regex = "1.12"

# Optional dependencies (only included when feature enabled)
pdfium-render = { version = "0.8.35", features = ["thread_safe", "image"], optional = true }
lopdf = { version = "0.38", optional = true }
calamine = { version = "0.31", features = ["dates"], optional = true }
polars = { version = "0.51", default-features = false, features = ["ipc"], optional = true }
mail-parser = { version = "0.11", optional = true }
msg_parser = { version = "0.1", optional = true }
html-to-markdown-rs = { path = "../../../html-to-markdown/crates/html-to-markdown", features = ["inline-images"], optional = true }
html-escape = { version = "0.2", optional = true }
quick-xml = { version = "0.38", optional = true }
roxmltree = { version = "0.21", optional = true }
zip = { version = "6.0", optional = true }
tar = { version = "0.4", optional = true }
sevenz-rust = { version = "0.6", optional = true }
tesseract-rs = { version = "0.1", optional = true }
image = { version = "0.25", default-features = false, features = ["png", "jpeg", "webp"], optional = true }
fast_image_resize = { version = "5.3", optional = true }
ndarray = { version = "0.17", optional = true }
whatlang = { version = "0.16", optional = true }
text-splitter = { version = "0.28", features = ["markdown"], optional = true }
unicode-normalization = { version = "0.1", optional = true }
chardetng = { version = "0.1", optional = true }
encoding_rs = { version = "0.8", optional = true }
axum = { version = "0.8", optional = true }
tower = { version = "0.5", optional = true }
tower-http = { version = "0.6", features = ["cors", "trace"], optional = true }
jsonrpc-core = { version = "18.0", optional = true }
pyo3 = { version = "0.23", features = ["auto-initialize"], optional = true }
```

---

## Conditional Compilation Strategy

### Lib.rs Structure

```rust
// Core (always available)
pub mod error;
pub mod types;
pub mod core;

// Format extractors (feature-gated)
#[cfg(feature = "pdf")]
pub mod pdf;

#[cfg(feature = "excel")]
pub mod extractors {
    pub mod excel;
}

#[cfg(feature = "ocr")]
pub mod ocr;

#[cfg(feature = "language-detection")]
pub mod language_detection;

#[cfg(feature = "chunking")]
pub mod chunking;

#[cfg(feature = "api")]
pub mod api;

#[cfg(feature = "mcp")]
pub mod mcp;

// Public API - conditionally export based on features
pub use core::{extract_file, extract_bytes, batch_extract_files, batch_extract_bytes};
pub use types::{ExtractionConfig, ExtractionResult, ExtractedTable};

#[cfg(feature = "api")]
pub use api::serve;

#[cfg(feature = "mcp")]
pub use mcp::serve_mcp;
```

### Registry Updates

```rust
// core/registry.rs
pub fn initialize_registry(config: &ExtractionConfig) -> Result<ExtractorRegistry> {
    let mut registry = ExtractorRegistry::new();

    // Core extractors (always available)
    registry.register("text/plain", Box::new(TextExtractor::new()))?;
    registry.register("text/markdown", Box::new(MarkdownExtractor::new()))?;
    registry.register("application/json", Box::new(JsonExtractor::new()))?;

    // Optional extractors
    #[cfg(feature = "pdf")]
    registry.register("application/pdf", Box::new(PdfExtractor::new()))?;

    #[cfg(feature = "excel")]
    registry.register("application/vnd.ms-excel", Box::new(ExcelExtractor::new()))?;

    #[cfg(feature = "email")]
    registry.register("message/rfc822", Box::new(EmailExtractor::new()))?;

    Ok(registry)
}
```

---

## Migration Path

### Phase 1: Introduce Features (Non-breaking)

1. Add feature flags to Cargo.toml
2. Keep `default = ["full"]` temporarily
3. Add `#[cfg(feature = "...")]` gates
4. Test compilation with each feature independently

### Phase 2: Update Defaults (Breaking)

1. Change `default = ["core-extractors"]`
2. Update documentation with migration guide
3. Release as v4.1.0 (minor version, but breaking for users relying on default)

### Phase 3: Optimize Dependencies

1. Split large dependencies into optional features
2. Reduce default feature set size
3. Benchmark binary sizes

---

## User Impact

### Before (v4.0)

```toml
[dependencies]
kreuzberg = "4.0"  # Gets everything, ~50MB binary
```

### After (v4.1)

```toml
[dependencies]
# Minimal - only text/JSON/YAML
kreuzberg = "4.0"  # ~10MB binary

# With PDFs
kreuzberg = { version = "4.0", features = ["pdf"] }  # ~35MB

# Full featured
kreuzberg = { version = "4.0", features = ["full"] }  # ~50MB

# Custom selection
kreuzberg = { version = "4.0", features = ["pdf", "excel", "ocr", "api"] }  # ~40MB
```

---

## Benefits

1. **Smaller binaries** - Users only pay for what they use
2. **Faster compilation** - Fewer dependencies to build
3. **Clearer documentation** - Feature matrix shows what's available
4. **Better for embedded** - Minimal default works on constrained systems
5. **Easier maintenance** - Features can be tested/developed independently

---

## Recommendations

### For Core Library

- ✅ Move language detection to `feature = "language-detection"`
- ✅ Add `feature = "api"` with Axum backend
- ✅ Add `feature = "mcp"` for Model Context Protocol
- ✅ Make OCR optional (`feature = "ocr"`)
- ✅ Make format extractors optional (PDF, Excel, etc.)

### For Python Package

- Python package always compiles with `features = ["full"]` (users expect everything)
- No need for `python-plugins` feature in `kreuzberg-py` (Python already present)
- Python optional dependencies remain (EasyOCR, PaddleOCR, spaCy)

### For CLI Binary

- Default to `features = ["cli"]` (common formats + OCR)
- Add `--features` flag to customize at compile time
- Document feature compilation in README

---

## Next Steps

1. Update `crates/kreuzberg/Cargo.toml` with feature flags
2. Add `#[cfg(feature = "...")]` gates throughout codebase
3. Move `language_detection.rs` behind feature gate
4. Create `crates/kreuzberg/src/api/` module
5. Create `crates/kreuzberg/src/mcp/` module
6. Update tests to compile with different feature combinations
7. Update TODO.md with implementation tasks
