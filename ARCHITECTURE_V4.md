# Kreuzberg v4 Architecture Design

## Executive Summary

This document outlines the new Python package structure for Kreuzberg v4, focusing on **zero-overhead Rust bindings** and **clean separation of concerns**.

______________________________________________________________________

## Core Principles

1. **Zero Serialization Overhead**: Direct PyO3 classes, no MessagePack in bindings
1. **Minimal Object Recreation**: Pass configs as kwargs or lightweight DTOs
1. **Core vs Features**: Rust-backed core, Python-only features
1. **Dataclass-Compatible**: Use PyO3 `#[pyclass]` that Python sees as objects with `__dataclass_fields__` if possible, else direct kwargs

______________________________________________________________________

## Package Structure

```text
kreuzberg/
├── __init__.py                    # Public API - re-exports from core + features
├── __main__.py                    # CLI entry point
├── cli.py                         # Click-based CLI
│
├── core/                          # Core: Thin wrappers around Rust (ZERO overhead)
│   ├── __init__.py                # Re-export main functions
│   ├── bindings.py                # Direct re-export from _internal_bindings
│   ├── extraction.py              # Main extraction functions (async/sync)
│   ├── types.py                   # Core types that mirror Rust 1:1
│   ├── config.py                  # Minimal config classes
│   ├── chunking.py                # Text chunking (Rust: TextSplitter, MarkdownSplitter)
│   ├── quality.py                 # Quality scoring (Rust: calculate_quality_score)
│   ├── cache.py                   # Caching system (Rust: GenericCache)
│   └── text_utils.py              # Text utilities (Rust: safe_decode, normalize_spaces, etc.)
│
├── features/                      # Features: Pure Python enhancements
│   ├── __init__.py
│   ├── api/                       # REST API (Litestar)
│   │   ├── __init__.py
│   │   ├── main.py
│   │   ├── routes.py
│   │   └── schemas.py
│   ├── language_detection.py      # fast-langdetect integration
│   ├── entity_extraction.py       # spaCy/transformers integration
│   ├── keyword_extraction.py      # YAKE integration
│   ├── hooks.py                   # Hook system (pre/post processing)
│   ├── validation.py              # Validation hooks
│   └── document_classification.py # ML-based classification
│
├── _extractors/                   # Extractors: Orchestrate Rust bindings
│   ├── __init__.py
│   ├── base.py                    # BaseExtractor (orchestration layer)
│   ├── pdf.py                     # PDF extraction (calls Rust)
│   ├── image.py                   # Image extraction + OCR
│   ├── email.py                   # Email extraction
│   ├── excel.py                   # Excel extraction
│   ├── pptx.py                    # PPTX extraction
│   ├── html.py                    # HTML extraction
│   ├── xml.py                     # XML extraction
│   ├── text.py                    # Plain text/markdown
│   ├── structured.py              # JSON/YAML/TOML
│   ├── pandoc.py                  # Pandoc-based extraction
│   └── legacy_office.py           # LibreOffice conversion
│
├── _ocr/                          # OCR: Orchestrate OCR backends
│   ├── __init__.py
│   ├── base.py                    # OCRBackend protocol
│   ├── tesseract.py               # Tesseract (Rust-backed)
│   ├── easyocr.py                 # EasyOCR (Python)
│   └── paddleocr.py               # PaddleOCR (Python)
│
├── _utils/                        # Utils: Helper functions
│   ├── __init__.py
│   ├── sync.py                    # Async/sync helpers
│   ├── ref.py                     # Reference holder pattern
│   ├── mime.py                    # MIME type detection
│   ├── registry.py                # Extractor registry
│   └── serialization.py           # msgspec helpers (for caching only)
│
└── exceptions.py                  # Exception classes
```

______________________________________________________________________

## Configuration Strategy

### Problem: Avoid Recreating Objects

**Current Issue**:

```text
# Python side
config = ExtractionConfig(target_dpi=150, max_image_dimension=25000, ...)

# Rust binding (BAD - full serialization)
config_msgpack = msgpack.encode(config)  # Serialize all 30+ fields
rust_function(config_msgpack)

# Rust side (BAD - full deserialization)
let config = msgpack::decode(bytes)?;  // Deserialize all 30+ fields, use only 5
```

### Solution 1: Direct Kwargs (Preferred for Simple Cases)

**When**: Function needs \<7 config parameters

```python
# Python side
def normalize_image_dpi(
    image: np.ndarray,
    target_dpi: int = 150,
    max_image_dimension: int = 25000,
    auto_adjust_dpi: bool = True,
    min_dpi: int = 72,
    max_dpi: int = 600,
    dpi_info: dict | None = None,
) -> tuple[np.ndarray, ImagePreprocessingMetadata]:
    return _internal_bindings.normalize_image_dpi(
        image,
        target_dpi,
        max_image_dimension,
        auto_adjust_dpi,
        min_dpi,
        max_dpi,
        dpi_info,
    )
```

**Rust Binding**:

```rust
#[pyfunction]
#[pyo3(signature = (image, target_dpi=150, max_image_dimension=25000, auto_adjust_dpi=true, min_dpi=72, max_dpi=600, dpi_info=None))]
pub fn normalize_image_dpi(
    image: PyReadonlyArray3<u8>,
    target_dpi: i32,
    max_image_dimension: i32,
    auto_adjust_dpi: bool,
    min_dpi: i32,
    max_dpi: i32,
    dpi_info: Option<&PyDict>,
) -> PyResult<(PyObject, PyImagePreprocessingMetadata)> {
    // Direct use - zero overhead!
    let config = kreuzberg::ImageConfig {
        target_dpi,
        max_image_dimension,
        auto_adjust_dpi,
        min_dpi,
        max_dpi,
    };
    // ...
}
```

**Benefits**:

- ✅ Zero serialization
- ✅ Zero object creation overhead
- ✅ Direct Python → Rust, no intermediate DTOs
- ✅ Clear API with defaults

______________________________________________________________________

### Solution 2: PyO3 Config Classes (for Complex Configs)

**When**: Function needs >7 parameters or config is reused multiple times

```rust
// crates/kreuzberg-py/src/types/config.rs
#[pyclass(name = "TesseractConfig")]
#[derive(Clone)]
pub struct PyTesseractConfig {
    #[pyo3(get, set)]
    pub language: String,
    #[pyo3(get, set)]
    pub psm: i32,
    #[pyo3(get, set)]
    pub output_format: String,
    #[pyo3(get, set)]
    pub enable_table_detection: bool,
    // ... 15+ more fields
}

#[pymethods]
impl PyTesseractConfig {
    #[new]
    #[pyo3(signature = (language="eng".to_string(), psm=3, output_format="markdown".to_string(), ...))]
    fn new(
        language: String,
        psm: i32,
        output_format: String,
        // ...
    ) -> Self {
        Self { language, psm, output_format, ... }
    }

    // Internal conversion to Rust core type
    pub(crate) fn to_core(&self) -> kreuzberg::ocr::TesseractConfig {
        kreuzberg::ocr::TesseractConfig {
            language: self.language.clone(),
            psm: self.psm,
            output_format: self.output_format.clone(),
            // ...
        }
    }
}
```

**Python Side**:

```python
# kreuzberg/core/config.py
from kreuzberg._internal_bindings import TesseractConfig

# Users can create and reuse configs
tesseract_config = TesseractConfig(language="eng", psm=3)

# Zero overhead passing to Rust
result = process_image(image, tesseract_config)
```

**Benefits**:

- ✅ Zero serialization when passed to Rust
- ✅ Reusable config objects
- ✅ Type-safe Python/Rust boundary
- ✅ Can add Python methods to config classes

**Tradeoff**:

- ❌ More Rust boilerplate (but worth it for complex configs)
- ❌ Two config systems (Python `ExtractionConfig` + PyO3 configs)

______________________________________________________________________

### Solution 3: Hybrid Approach (Recommended)

**Python High-Level Config** (user-facing):

```python
# kreuzberg/core/config.py
@dataclass(frozen=True, kw_only=True)
class ExtractionConfig:
    """High-level configuration - user-facing API"""

    # OCR (tagged union)
    ocr: TesseractConfig | EasyOCRConfig | PaddleOCRConfig | None = None

    # Features
    chunking: ChunkingConfig | None = None
    language_detection: LanguageDetectionConfig | None = None

    # DPI settings (for Rust bindings)
    target_dpi: int = 150
    max_image_dimension: int = 25000
    auto_adjust_dpi: bool = True
    min_dpi: int = 72
    max_dpi: int = 600

    # PDF settings
    pdf_password: str | tuple[str, ...] = ""

    # Global
    use_cache: bool = True
    enable_quality_processing: bool = True
```

**Conversion Helpers** (internal):

```python
# kreuzberg/core/extraction.py
def _extract_image_kwargs(config: ExtractionConfig) -> dict:
    """Extract image-related kwargs for Rust bindings"""
    return {
        "target_dpi": config.target_dpi,
        "max_image_dimension": config.max_image_dimension,
        "auto_adjust_dpi": config.auto_adjust_dpi,
        "min_dpi": config.min_dpi,
        "max_dpi": config.max_dpi,
    }

def _extract_tesseract_config(config: ExtractionConfig) -> PyTesseractConfig | None:
    """Convert Python TesseractConfig to PyO3 TesseractConfig"""
    if not isinstance(config.ocr, TesseractConfig):
        return None

    return PyTesseractConfig(
        language=config.ocr.language,
        psm=config.ocr.psm.value if hasattr(config.ocr.psm, "value") else config.ocr.psm,
        output_format=config.ocr.output_format,
        # ...
    )
```

**Usage in Extractors**:

```python
# kreuzberg/_extractors/image.py
class ImageExtractor(BaseExtractor):
    def extract_bytes_sync(self, content: bytes) -> ExtractionResult:
        # Normalize DPI using kwargs
        image_kwargs = _extract_image_kwargs(self.config)
        normalized_image, metadata = normalize_image_dpi(image_array, **image_kwargs, dpi_info=dpi_info)

        # OCR using PyO3 config
        if tesseract_config := _extract_tesseract_config(self.config):
            ocr_result = process_image(normalized_image, tesseract_config)
```

**Benefits**:

- ✅ Clean user-facing API (single `ExtractionConfig`)
- ✅ Zero overhead Rust calls (kwargs or PyO3 classes)
- ✅ No intermediate DTOs or serialization
- ✅ Flexibility to optimize hot paths

______________________________________________________________________

## PyO3 Class Design Pattern

### For Result Classes (Read-Only)

```rust
// crates/kreuzberg-py/src/types/email.rs
use pyo3::prelude::*;

#[pyclass(name = "EmailExtractionResult", module = "kreuzberg._internal_bindings")]
#[derive(Clone)]
pub struct PyEmailExtractionResult {
    pub(crate) inner: kreuzberg::types::EmailExtractionResult,
}

#[pymethods]
impl PyEmailExtractionResult {
    // Read-only properties (no set)
    #[getter]
    fn subject(&self) -> Option<String> {
        self.inner.subject.clone()
    }

    #[getter]
    fn from_email(&self) -> Option<String> {
        self.inner.from_email.clone()
    }

    #[getter]
    fn to_emails(&self) -> Vec<String> {
        self.inner.to_emails.clone()
    }

    #[getter]
    fn attachments(&self) -> Vec<PyEmailAttachment> {
        self.inner.attachments
            .iter()
            .map(|a| PyEmailAttachment::from(a.clone()))
            .collect()
    }

    #[getter]
    fn metadata(&self) -> HashMap<String, String> {
        self.inner.metadata.clone()
    }

    // Optional: Helper method for compatibility
    fn to_dict(&self) -> HashMap<String, PyObject> {
        Python::with_gil(|py| {
            let mut dict = HashMap::new();
            dict.insert("subject", self.subject().to_object(py));
            dict.insert("from_email", self.from_email().to_object(py));
            // ...
            dict
        })
    }
}

// Zero-cost conversion from Rust type
impl From<kreuzberg::types::EmailExtractionResult> for PyEmailExtractionResult {
    fn from(inner: kreuzberg::types::EmailExtractionResult) -> Self {
        Self { inner }
    }
}
```

______________________________________________________________________

### For Config Classes (Read-Write)

```rust
// crates/kreuzberg-py/src/types/config.rs
#[pyclass(name = "TesseractConfig", module = "kreuzberg._internal_bindings")]
#[derive(Clone)]
pub struct PyTesseractConfig {
    #[pyo3(get, set)]
    pub language: String,

    #[pyo3(get, set)]
    pub psm: i32,

    #[pyo3(get, set)]
    pub output_format: String,

    #[pyo3(get, set)]
    pub enable_table_detection: bool,

    // ... more fields
}

#[pymethods]
impl PyTesseractConfig {
    #[new]
    #[pyo3(signature = (
        language="eng".to_string(),
        psm=3,
        output_format="markdown".to_string(),
        enable_table_detection=true,
        // ...
    ))]
    fn new(
        language: String,
        psm: i32,
        output_format: String,
        enable_table_detection: bool,
        // ...
    ) -> Self {
        Self {
            language,
            psm,
            output_format,
            enable_table_detection,
            // ...
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "TesseractConfig(language={}, psm={}, output_format={}, ...)",
            self.language, self.psm, self.output_format
        )
    }

    // Internal conversion to Rust core type
    pub(crate) fn to_core(&self) -> kreuzberg::ocr::TesseractConfig {
        kreuzberg::ocr::TesseractConfig {
            language: self.language.clone(),
            psm: self.psm,
            output_format: self.output_format.clone(),
            enable_table_detection: self.enable_table_detection,
            // ...
        }
    }
}
```

**Python Usage**:

```python
from kreuzberg._internal_bindings import TesseractConfig

# Create config
config = TesseractConfig(language="eng", psm=3)

# Read/write properties
print(config.language)  # "eng"
config.language = "fra"  # Mutable!

# Pass directly to Rust
result = process_image(image, config)  # Zero overhead!
```

______________________________________________________________________

## Core Module Design

### `kreuzberg/core/extraction.py`

**Philosophy**: Thin orchestration layer - delegates to Rust

```python
"""Core extraction functions - thin wrappers around Rust bindings"""

from pathlib import Path
from typing import AsyncIterator

from kreuzberg.core.bindings import (
    extract_email_content,
    extract_html_content,
    parse_xml,
    parse_text,
    read_excel_bytes,
    # ... all Rust functions
)
from kreuzberg.core.types import ExtractionResult, ExtractionConfig
from kreuzberg._extractors import ExtractorRegistry
from kreuzberg.features.hooks import apply_post_processing_hooks

async def extract_file(
    file_path: str | Path,
    mime_type: str | None = None,
    config: ExtractionConfig | None = None,
) -> ExtractionResult:
    """Extract content from a file (async)

    This is the main entry point. It:
    1. Validates mime type
    2. Gets appropriate extractor from registry
    3. Calls extractor (which calls Rust bindings)
    4. Applies post-processing features (Python)
    5. Returns result
    """
    config = config or ExtractionConfig()
    path = Path(file_path)

    # Validate MIME type
    mime_type = mime_type or _detect_mime_type(path)

    # Get extractor (orchestration layer)
    extractor = ExtractorRegistry.get_extractor(mime_type, config)

    # Extract (calls Rust bindings)
    result = await extractor.extract_path_async(path)

    # Apply Python features
    result = await _apply_features(result, config)

    return result

async def _apply_features(
    result: ExtractionResult,
    config: ExtractionConfig,
) -> ExtractionResult:
    """Apply Python-side features (language detection, entity extraction, etc.)"""

    # Chunking (Rust-backed via TextSplitter)
    if config.chunking:
        result = await _apply_chunking(result, config.chunking)

    # Language detection (Python - fast-langdetect)
    if config.language_detection:
        from kreuzberg.features.language_detection import detect_language

        result.detected_languages = await detect_language(result.content, config.language_detection)

    # Entity extraction (Python - spaCy/transformers)
    if config.entities:
        from kreuzberg.features.entity_extraction import extract_entities

        result.entities = await extract_entities(result.content, config.entities)

    # Post-processing hooks (Python)
    if config.post_processing_hooks:
        from kreuzberg.features.hooks import apply_hooks

        result = await apply_hooks(result, config.post_processing_hooks)

    return result
```

______________________________________________________________________

### `kreuzberg/core/bindings.py`

**Philosophy**: Direct re-export, zero overhead

```python
"""Direct re-exports from Rust bindings - zero overhead"""

# Email
from kreuzberg._internal_bindings import (
    EmailExtractionResult,
    EmailAttachment,
    extract_email_content,
    parse_eml_content,
    parse_msg_content,
    build_email_text_output,
)

# Excel
from kreuzberg._internal_bindings import (
    ExcelWorkbook,
    ExcelSheet,
    read_excel_bytes,
    read_excel_file,
)

# HTML
from kreuzberg._internal_bindings import (
    HtmlExtractionResult,
    ExtractedInlineImage,
    convert_html_to_markdown,
    process_html,
)

# XML
from kreuzberg._internal_bindings import (
    XmlExtractionResult,
    parse_xml,
)

# Text
from kreuzberg._internal_bindings import (
    TextExtractionResult,
    parse_text,
)

# Chunking (Rust-backed)
from kreuzberg._internal_bindings import (
    TextSplitter,
    MarkdownSplitter,
)

# Quality (Rust-backed)
from kreuzberg._internal_bindings import (
    calculate_quality_score,
    clean_extracted_text,
    normalize_spaces,
)

# Cache (Rust-backed)
from kreuzberg._internal_bindings import (
    GenericCache,
    generate_cache_key,
    batch_generate_cache_keys,
)

# Text utilities (Rust-backed)
from kreuzberg._internal_bindings import (
    safe_decode,
    fix_mojibake,
    batch_process_texts,
)

# Table utilities (Rust-backed)
from kreuzberg._internal_bindings import (
    table_from_arrow_to_markdown,
)

__all__ = [
    # Email
    "EmailExtractionResult",
    "EmailAttachment",
    "extract_email_content",
    # ... all exports
]
```

______________________________________________________________________

### `kreuzberg/core/chunking.py`

**Philosophy**: Thin wrapper around Rust TextSplitter/MarkdownSplitter

```python
"""Text chunking - Rust-backed via text-splitter crate"""

from dataclasses import dataclass

from kreuzberg._internal_bindings import TextSplitter, MarkdownSplitter

@dataclass(frozen=True, kw_only=True)
class ChunkingConfig:
    """Configuration for text chunking"""

    max_chars: int = 2000
    overlap: int = 100
    trim: bool = True
    chunker_type: str = "text"  # "text" or "markdown"

def chunk_text(text: str, config: ChunkingConfig | None = None) -> list[str]:
    """Chunk text using Rust-backed splitter (zero overhead)"""
    config = config or ChunkingConfig()

    if config.chunker_type == "markdown":
        splitter = MarkdownSplitter(
            max_characters=config.max_chars,
            overlap=config.overlap,
            trim=config.trim,
        )
    else:
        splitter = TextSplitter(
            max_characters=config.max_chars,
            overlap=config.overlap,
            trim=config.trim,
        )

    return splitter.chunks(text)
```

______________________________________________________________________

## Features Module Design

### `kreuzberg/features/api/main.py`

**Philosophy**: REST API for extraction - uses core functions

```python
"""REST API using Litestar - delegates to core.extraction"""

from litestar import Litestar, post, get
from litestar.datastructures import UploadFile

from kreuzberg.core.extraction import extract_file_sync, extract_bytes_sync
from kreuzberg.core.types import ExtractionConfig, ExtractionResult

@post("/extract/file")
async def extract_file_endpoint(
    file: UploadFile,
    config: ExtractionConfig | None = None,
) -> ExtractionResult:
    """Extract content from uploaded file"""
    content = await file.read()
    mime_type = file.content_type

    return extract_bytes_sync(content, mime_type, config or ExtractionConfig())

app = Litestar(route_handlers=[extract_file_endpoint])
```

______________________________________________________________________

### `kreuzberg/features/language_detection.py`

**Philosophy**: Pure Python feature using fast-langdetect

```python
"""Language detection using fast-langdetect"""

from dataclasses import dataclass
from fast_langdetect import detect_multilingual

@dataclass(frozen=True, kw_only=True)
class LanguageDetectionConfig:
    top_k: int = 3
    multilingual: bool = False

async def detect_language(
    text: str,
    config: LanguageDetectionConfig,
) -> list[tuple[str, float]]:
    """Detect language(s) in text"""
    if config.multilingual:
        results = detect_multilingual(text, k=config.top_k)
    else:
        results = [detect(text)]

    return [(r.lang, r.score) for r in results]
```

______________________________________________________________________

## Migration Path

### Phase 1: Create Core Structure (Week 1)

1. Create `kreuzberg/core/` directory structure
1. Create `kreuzberg/core/bindings.py` - direct re-exports
1. Create `kreuzberg/core/types.py` - mirror current types
1. Create `kreuzberg/core/extraction.py` - refactor from `extraction.py`
1. Update `kreuzberg/__init__.py` to re-export from core

### Phase 2: Migrate PPTX to PyO3 (Week 1-2)

1. Create `crates/kreuzberg-py/src/types/pptx.rs`
1. Update `crates/kreuzberg-py/src/bindings/pptx.rs`
1. Update `kreuzberg/_extractors/_presentation.py`
1. Add deprecation warnings to `*_msgpack` functions

### Phase 3: Create Features Structure (Week 2)

1. Create `kreuzberg/features/` directory
1. Move API to `kreuzberg/features/api/`
1. Move language detection to `kreuzberg/features/language_detection.py`
1. Move entity extraction to `kreuzberg/features/entity_extraction.py`

### Phase 4: Optimize Config Passing (Week 2-3)

1. Convert image preprocessing to kwargs
1. Create PyO3 TesseractConfig
1. Update all Rust bindings to use new patterns

### Phase 5: Complete MessagePack Removal (Week 3-4)

1. Migrate structured data extraction
1. Migrate Pandoc bindings
1. Migrate LibreOffice bindings
1. Remove all `*_msgpack` functions
1. Update documentation

______________________________________________________________________

## Performance Goals

| Operation           | Current              | Target | Improvement |
| ------------------- | -------------------- | ------ | ----------- |
| PPTX extraction     | 50-200μs overhead    | \<1μs  | 50-200x     |
| Config passing      | 5-20μs serialization | 0μs    | ∞           |
| Image preprocessing | 10-30μs overhead     | 0μs    | ∞           |
| Structured data     | 20-80μs overhead     | \<1μs  | 20-80x      |

______________________________________________________________________

## API Stability Guarantee

**Public API** (in `kreuzberg/__init__.py`):

- ✅ No breaking changes
- ✅ `extract_file()`, `extract_bytes()` signatures unchanged
- ✅ `ExtractionConfig` signature unchanged
- ✅ `ExtractionResult` structure unchanged

**Internal API** (in `kreuzberg.core`, `kreuzberg.features`):

- ⚠️ May change between minor versions
- ⚠️ Users should not depend on internal APIs

______________________________________________________________________

## Testing Strategy

### Unit Tests

- Test each extractor independently
- Test each feature independently
- Test config conversion helpers

### Integration Tests

- Test full extraction pipeline
- Test with real documents
- Test with various configs

### Performance Tests

- Benchmark config passing overhead
- Benchmark serialization elimination
- Compare before/after migration

______________________________________________________________________

## Next Steps

1. **Review this document** - Get feedback on approach
1. **Start Phase 1** - Create core structure
1. **Implement Phase 2** - Migrate PPTX to PyO3
1. **Measure performance** - Validate improvements
1. **Continue phases** - Systematic migration
