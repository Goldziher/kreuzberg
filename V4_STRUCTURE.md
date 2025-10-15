# Kreuzberg V4 Architecture: Rust-First Design

## Core Philosophy

**Design Principles:**

- **Rust Core**: All orchestration, routing, caching, MIME detection, and pipeline logic implemented in Rust
- **Language-Agnostic Plugins**: Trait-based plugin system that works across FFI boundaries (Python, Node, Rust)
- **Thin Bindings**: Python/Node packages are minimal wrappers that proxy to Rust core
- **Performance**: Zero-copy data passing, efficient serialization, native parallelism

**Migration Strategy:**

- Old `src/` structure → **DELETED**
- Current `kreuzberg/` Python package → will become `kreuzberg_legacy/` → eventually **DELETED**
- `crates/kreuzberg/` → **Core Rust library** (the new heart of the system)
- `crates/kreuzberg-py/` → Thin Python bindings that proxy to Rust core

______________________________________________________________________

## 1. Rust Core Library Structure

```
crates/kreuzberg/src/
├── lib.rs                    # Public Rust API
├── error.rs                  # Unified error types (KreuzbergError hierarchy)
├── types.rs                  # Core types (ExtractionResult, Config, etc.)
│
├── core/                     # ★ Core Orchestration (NEW)
│   ├── mod.rs
│   ├── extractor.rs         # Main entry: extract_file(), extract_bytes()
│   ├── registry.rs          # Maps MIME types → extractors, manages plugins
│   ├── mime.rs              # MIME detection & validation
│   ├── pipeline.rs          # Post-processing pipeline orchestration
│   ├── config.rs            # Config loading (TOML/YAML/JSON, discovery)
│   └── io.rs                # File I/O utilities
│
├── plugins/                  # ★ Plugin System (NEW)
│   ├── mod.rs
│   ├── traits.rs            # Base Plugin trait
│   ├── ocr.rs               # OcrBackend trait
│   ├── extractor.rs         # DocumentExtractor trait
│   ├── processor.rs         # PostProcessor trait
│   ├── validator.rs         # Validator trait
│   ├── registry.rs          # Plugin registry & discovery
│   └── ffi.rs               # FFI bridge utilities for cross-language plugins
│
├── extraction/               # Built-in extractors (existing, refactored to use traits)
│   ├── mod.rs
│   ├── pdf.rs               # PDF extraction (pdfium)
│   ├── excel.rs             # Excel/spreadsheets (calamine)
│   ├── email.rs             # Email (EML/MSG with mail-parser)
│   ├── html.rs              # HTML (html-to-markdown)
│   ├── pptx.rs              # PowerPoint
│   ├── xml.rs               # XML streaming parser
│   ├── text.rs              # Plain text/markdown streaming parser
│   ├── image.rs             # Images
│   ├── structured.rs        # JSON/YAML/TOML
│   ├── pandoc.rs            # Pandoc integration (subprocess)
│   └── libreoffice.rs       # LibreOffice conversion (subprocess)
│
├── ocr/                      # OCR subsystem
│   ├── mod.rs
│   ├── processor.rs         # OCR orchestration logic
│   ├── tesseract.rs         # Native Tesseract backend implementation
│   ├── plugin_bridge.rs     # ★ Bridge for Python/Node OCR backends (FFI)
│   ├── types.rs             # OCR-specific types
│   ├── cache.rs             # OCR result caching
│   ├── hocr.rs              # HOCR parsing
│   ├── validation.rs        # OCR validation
│   └── table/               # Table detection from OCR
│       ├── mod.rs
│       ├── detection.rs
│       ├── reconstruction.rs
│       └── tsv_parser.rs
│
├── processing/               # Built-in post-processors
│   ├── mod.rs
│   ├── chunking.rs          # Text chunking (text-splitter)
│   ├── quality.rs           # Quality scoring
│   ├── token_reduction.rs   # Token reduction
│   └── plugin_bridge.rs     # ★ Bridge for language-specific processors (spaCy, etc.)
│
├── pdf/                      # PDF utilities
│   ├── mod.rs
│   ├── text.rs              # Text extraction from PDFs
│   ├── images.rs            # Image extraction from PDFs
│   ├── metadata.rs          # PDF metadata
│   ├── rendering.rs         # PDF rendering (pdfium)
│   └── error.rs
│
├── image/                    # Image utilities
│   ├── mod.rs
│   ├── preprocessing.rs     # DPI normalization, resizing
│   ├── compression.rs
│   └── conversions.rs
│
├── text/                     # Text utilities
│   ├── mod.rs
│   ├── encoding.rs          # Character encoding detection/conversion
│   ├── quality.rs           # Text quality scoring
│   └── normalization.rs     # Unicode normalization
│
└── cache/                    # Caching system
    ├── mod.rs
    └── document_cache.rs
```

______________________________________________________________________

## 2. Plugin System Design

### Core Plugin Traits

#### Base Plugin Trait

```rust
pub trait Plugin: Send + Sync {
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    fn initialize(&mut self) -> Result<()>;
    fn shutdown(&mut self) -> Result<()>;
}
```

#### OCR Backend Plugin

```rust
#[async_trait]
pub trait OcrBackend: Plugin {
    async fn process_image(&self, image: &Image, config: &OcrConfig) -> Result<ExtractionResult>;
    async fn process_file(&self, path: &Path, config: &OcrConfig) -> Result<ExtractionResult>;
    fn supports_language(&self, lang: &str) -> bool;
    fn backend_type(&self) -> OcrBackendType; // tesseract, easyocr, paddleocr
}
```

#### Document Extractor Plugin

```rust
#[async_trait]
pub trait DocumentExtractor: Plugin {
    async fn extract_bytes(&self, content: &[u8], mime_type: &str, config: &ExtractionConfig)
        -> Result<ExtractionResult>;
    async fn extract_file(&self, path: &Path, mime_type: &str, config: &ExtractionConfig)
        -> Result<ExtractionResult>;
    fn supported_mime_types(&self) -> &[&str];
    fn priority(&self) -> i32; // For handling overlapping MIME types
}
```

#### Post-Processor Plugin

```rust
#[async_trait]
pub trait PostProcessor: Plugin {
    async fn process(&self, result: ExtractionResult, config: &ExtractionConfig)
        -> Result<ExtractionResult>;
    fn processing_stage(&self) -> ProcessingStage; // early, middle, late
}
```

#### Validator Plugin

```rust
#[async_trait]
pub trait Validator: Plugin {
    async fn validate(&self, result: &ExtractionResult, config: &ExtractionConfig) -> Result<()>;
}
```

### Plugin Communication Across Languages

Plugins can be implemented in:

- **Rust** (native, fastest)
- **Python** (via PyO3 FFI bridge)
- **Node.js** (via napi-rs FFI bridge - future)

Example FFI Bridge for Python OCR Backend:

```rust
// Rust side
pub struct PythonOcrBackend {
    callback: extern "C" fn(*const u8, usize, *const OcrConfig) -> *mut ExtractionResult,
    name: String,
}

#[async_trait]
impl OcrBackend for PythonOcrBackend {
    async fn process_image(&self, image: &Image, config: &OcrConfig) -> Result<ExtractionResult> {
        // Call Python callback via FFI
        let result_ptr = (self.callback)(image.as_ptr(), image.len(), config as *const _);
        unsafe { Ok(*Box::from_raw(result_ptr)) }
    }
}
```

______________________________________________________________________

## 3. Core Extraction Flow

### Main Entry Point (`core/extractor.rs`)

```rust
pub async fn extract_file(
    path: impl AsRef<Path>,
    mime_type: Option<&str>,
    config: &ExtractionConfig,
) -> Result<ExtractionResult> {
    let path = path.as_ref();

    // 1. Cache check
    if config.use_cache {
        if let Some(cached) = cache::get(path, config).await? {
            return Ok(cached);
        }
    }

    // 2. MIME detection
    let mime_type = mime::detect_or_validate(path, mime_type)?;

    // 3. Get extractor from registry
    let extractor = registry::get_extractor(&mime_type, config)?;

    // 4. Extract content
    let mut result = extractor.extract_file(path, &mime_type, config).await?;

    // 5. Run post-processing pipeline
    result = pipeline::run_pipeline(result, config).await?;

    // 6. Cache result
    if config.use_cache {
        cache::set(path, config, &result).await?;
    }

    Ok(result)
}
```

### Registry (`core/registry.rs`)

- Maps MIME types to extractors
- Manages plugin registration and discovery
- Handles priority-based extractor selection
- Thread-safe plugin registry with RwLock

### Pipeline (`core/pipeline.rs`)

Orchestrates post-processing in stages:

1. **Validators** - Run validation hooks
1. **Quality Processing** - Text cleaning, quality scoring
1. **Chunking** - Text splitting
1. **Post-Processors** (by stage):
    - Early: Language detection, entity extraction
    - Middle: Keyword extraction, token reduction
    - Late: Custom user hooks
1. **Custom Hooks** - User-registered plugins

______________________________________________________________________

## 4. Configuration System

### Rust Config (`core/config.rs`)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionConfig {
    // Core
    pub use_cache: bool,
    pub enable_quality_processing: bool,

    // OCR
    pub ocr: Option<OcrConfig>,
    pub force_ocr: bool,

    // Features
    pub chunking: Option<ChunkingConfig>,
    pub tables: Option<TableConfig>,
    pub images: Option<ImageConfig>,
    pub language_detection: Option<LanguageDetectionConfig>,
    pub entities: Option<EntityConfig>,
    pub keywords: Option<KeywordConfig>,

    // Hooks (stored as plugin IDs, actual plugins in registry)
    pub validators: Vec<String>,
    pub post_processors: Vec<String>,
}
```

### Config Discovery

- Search for `kreuzberg.toml` in current and parent directories
- Support TOML, YAML, and JSON formats via explicit file loading
- Environment variable overrides
- **Breaking Change (V4)**: No longer supports `pyproject.toml` in Rust core
    - Python bindings layer can add `pyproject.toml` support if needed
    - Keeps Rust core language-agnostic

______________________________________________________________________

## 5. Python Bindings Architecture

### Thin Proxy Layer (`crates/kreuzberg-py/`)

**Old Approach (WRONG):**

- Separate PyO3 bindings for each extractor
- Duplicate Python logic
- MessagePack serialization overhead

**New Approach (CORRECT):**

```python
# kreuzberg/__init__.py
from kreuzberg._internal_bindings import (
    extract_file as _extract_file_rust,
    extract_bytes as _extract_bytes_rust,
)

def extract_file(path: str, mime_type: str | None = None, config: ExtractionConfig | None = None) -> ExtractionResult:
    """Extract content from a file (proxies to Rust core)."""
    config = config or ExtractionConfig()
    return _extract_file_rust(str(path), mime_type, config)

# Python-specific OCR backends register as plugins with Rust core
from kreuzberg._ocr._easyocr import EasyOCRBackend
from kreuzberg._ocr._paddleocr import PaddleBackend

_internal_bindings.register_ocr_backend("easyocr", EasyOCRBackend())
_internal_bindings.register_ocr_backend("paddleocr", PaddleBackend())
```

### Python Plugin Registration

Python implementations of `OcrBackend`, `PostProcessor`, etc. are registered with the Rust core at module import time. The Rust core stores function pointers and calls back into Python when needed.

______________________________________________________________________

## 6. Error Handling

### Rust Error Types (`error.rs`)

```rust
#[derive(Debug, thiserror::Error)]
pub enum KreuzbergError {
    #[error("Validation error: {message}")]
    Validation { message: String, context: serde_json::Value },

    #[error("Parsing error: {message}")]
    Parsing { message: String, context: serde_json::Value },

    #[error("OCR error: {message}")]
    Ocr { message: String, context: serde_json::Value },

    #[error("Missing dependency: {dependency}")]
    MissingDependency { dependency: String, install_hint: String },

    #[error("Plugin error: {message}")]
    Plugin { message: String, plugin_name: String },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Unsupported MIME type: {0}")]
    UnsupportedMimeType(String),
}

// CRITICAL: OSError and RuntimeError equivalents ALWAYS bubble up
impl KreuzbergError {
    pub fn should_bubble_up(&self) -> bool {
        matches!(self,
            KreuzbergError::Io(_) |
            KreuzbergError::Plugin { .. }
        )
    }
}
```

### Error Propagation Rules

- **System errors** (IO, RuntimeError equivalents) → **ALWAYS BUBBLE UP**
- **Parsing errors** → Wrap with context, convert to `ParsingError`
- **Optional feature errors** → Catch, log, continue with degraded result
- **Plugin errors** → Bubble up with plugin context for debugging

______________________________________________________________________

## 7. Key Advantages

1. **Single Source of Truth**: All logic in Rust = consistent behavior across Python, Node, CLI
1. **Performance**: Native Rust speed for orchestration, no Python overhead
1. **Extensibility**: Plugin system allows community extensions in any language
1. **Maintainability**: One codebase to maintain, bindings are auto-generated boilerplate
1. **Type Safety**: Rust's type system prevents entire classes of bugs
1. **Async Native**: Tokio-based async throughout, efficient I/O
1. **Cross-Platform**: Rust compile once, run anywhere (including WASM future)

______________________________________________________________________

## 8. Migration Phases

### Phase 1: Core Infrastructure (Current Focus)

- ✅ Create `core/` module structure
- ✅ Implement main extraction entry points
- ✅ Implement registry for extractors
- ✅ Implement MIME detection/validation
- ✅ Implement pipeline orchestration
- ✅ Implement config loading and discovery

### Phase 2: Plugin System

- Implement base plugin traits
- Implement FFI bridge utilities
- Port existing extractors to plugin trait system
- Create plugin registry with priority support

### Phase 3: Extractor Migration

- Migrate all built-in extractors to new trait system
- Ensure all extractors implement `DocumentExtractor` trait
- Test extractor registration and priority handling

### Phase 4: Python Bindings Rewrite

- Create thin proxy layer in Python
- Implement Python OCR backend bridge (EasyOCR, PaddleOCR)
- Implement Python post-processor bridge (spaCy, langdetect)
- Remove old PyO3 extractor bindings

### Phase 5: Testing & Optimization

- Port all tests to new system
- Performance benchmarking
- Memory profiling
- Documentation updates

______________________________________________________________________

## 9. Design Decisions

### OCR Backend Strategy

**Decision**: Tesseract is the only **native Rust** OCR backend. EasyOCR and PaddleOCR remain Python-side plugins.

**Rationale**:

- Tesseract has Rust bindings (`tesseract-rs`)
- EasyOCR/PaddleOCR are Python-first, no quality Rust alternatives
- Plugin system allows Python OCR backends with minimal overhead

### Async Strategy

**Decision**: Full async/await throughout Rust core with sync wrappers at API level.

**Rationale**:

- Async enables efficient concurrent processing (batch operations)
- Tokio runtime provides excellent performance
- Sync wrappers trivial to add (`block_on`)

### Plugin Discovery

**Decision**: Explicit registration at module import time (no auto-discovery).

**Rationale**:

- Explicit is better than implicit
- Avoids complex classpath scanning
- Clear dependency management

### Serialization Format

**Decision**: Native Rust types for internal communication, MessagePack for FFI boundaries.

**Rationale**:

- Zero-copy within Rust
- MessagePack compact and fast for FFI
- Avoids JSON parsing overhead

### Config Extensions

**Decision**: Plugins **cannot** extend `ExtractionConfig`. Use metadata fields instead.

**Rationale**:

- Maintains type safety
- Avoids config schema fragmentation
- Plugins can use `metadata: HashMap<String, Value>` for custom fields

### Node.js Priority

**Decision**: Build Node bindings **after** Python is stable.

**Rationale**:

- Validate architecture with Python first
- napi-rs provides similar FFI patterns to PyO3
- Learn from Python binding mistakes

______________________________________________________________________

## 10. File Naming Conventions

- **Rust**: `snake_case.rs`
- **Modules**: `mod.rs` for module root
- **Tests**: `#[cfg(test)]` in same file or `tests/` directory
- **Examples**: `examples/*.rs`

______________________________________________________________________

## 11. Performance Goals

- **Extraction Speed**: 40-70% faster than V4 MessagePack approach
- **Memory Usage**: 30% lower than Python-only approach
- **Startup Time**: \<100ms cold start (Rust binary)
- **Concurrency**: Linear scaling up to CPU core count

______________________________________________________________________

## 12. Future Enhancements

- **WASM Support**: Compile Rust core to WebAssembly for browser use
- **Distributed Processing**: Plugin-based distributed extraction
- **GPU Acceleration**: Plugin for GPU-based table detection
- **Streaming API**: Process large files incrementally
- **gRPC Server**: Alternative to REST API for performance

______________________________________________________________________

## Questions & Open Issues

1. **Async in Python Plugins**: Should Python plugin callbacks support async?
1. **Plugin Versioning**: How to handle plugin API version compatibility?
1. **Hot Reloading**: Should plugins support hot reloading without restart?
1. **Resource Limits**: Should plugins declare resource requirements (memory, CPU)?
1. **Sandboxing**: Should plugins run in isolated environments for security?

______________________________________________________________________

**Last Updated**: 2025-10-15
**Status**: Active Development - Phase 1 in progress
