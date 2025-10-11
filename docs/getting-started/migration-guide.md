# Migration Guide: v3.x to v4.0

This guide helps you migrate from Kreuzberg v3.x to v4.0, which introduces **major breaking changes** in configuration structure, type system, and API design.

## ⚠️ Breaking Changes Overview

Version 4.0 is a **complete rewrite** of the configuration system with:

- **No backward compatibility** - V3 configs will not work
- **New type system** - msgspec.Struct instead of dataclasses
- **Nested configuration** - Object-oriented config design
- **Tagged unions** - OCR backend selection via config type
- **Immutable collections** - Tuples instead of lists
- **Python 3.10+** - Modern union syntax required

## Configuration Structure Changes

### 1. OCR Configuration - Tagged Union Design

**V3.x (Old):**

```python
from kreuzberg import ExtractionConfig, TesseractConfig

# Separate backend selection and config
config = ExtractionConfig(
    ocr_backend="tesseract",  # String field for backend selection
    ocr_config=TesseractConfig(language="eng"),  # Separate config object
)
```

**V4.0 (New):**

```python
from kreuzberg import ExtractionConfig, TesseractConfig

# Tagged union - config type IS the backend selection
config = ExtractionConfig(
    ocr=TesseractConfig(language="eng"),  # Config type determines backend
)
```

**Key Changes:**

- `ocr_backend` field removed
- `ocr_config` renamed to `ocr`
- Backend determined by config object type (TesseractConfig, EasyOCRConfig, PaddleOCRConfig)
- Set `ocr=None` to disable OCR

**All OCR Backends:**

```python
# Tesseract
config = ExtractionConfig(ocr=TesseractConfig(language="eng"))

# EasyOCR
config = ExtractionConfig(ocr=EasyOCRConfig(language="en"))

# PaddleOCR
config = ExtractionConfig(ocr=PaddleOCRConfig(language="en"))

# Disabled
config = ExtractionConfig(ocr=None)
```

### 2. Feature Flags → Config Objects

All boolean feature flags are replaced with config objects.

**V3.x (Old):**

```python
config = ExtractionConfig(
    extract_tables=True,  # Boolean flag
    extract_images=True,  # Boolean flag
    extract_keywords=True,  # Boolean flag
    extract_entities=True,  # Boolean flag
    chunk_content=True,  # Boolean flag
    auto_detect_language=True,  # Boolean flag
)
```

**V4.0 (New):**

```python
from kreuzberg import (
    ExtractionConfig,
    TableExtractionConfig,
    ImageExtractionConfig,
    KeywordExtractionConfig,
    EntityExtractionConfig,
    ChunkingConfig,
    LanguageDetectionConfig,
)

config = ExtractionConfig(
    tables=TableExtractionConfig(),  # Config object (enabled)
    images=ImageExtractionConfig(),  # Config object (enabled)
    keywords=KeywordExtractionConfig(),  # Config object (enabled)
    entities=EntityExtractionConfig(),  # Config object (enabled)
    chunking=ChunkingConfig(),  # Config object (enabled)
    language_detection=LanguageDetectionConfig(),  # Config object (enabled)
)
```

**Disabled Features:**

```python
# In V3: extract_tables=False
# In V4: tables=None (or omit the field)

config = ExtractionConfig(
    tables=None,  # Disabled
    images=None,  # Disabled
)
```

### 3. Chunking Configuration

**V3.x (Old):**

```python
config = ExtractionConfig(
    chunk_content=True,
    max_chars=1000,
    max_overlap=200,
)
```

**V4.0 (New):**

```python
config = ExtractionConfig(
    chunking=ChunkingConfig(
        max_chars=1000,
        max_overlap=200,
    ),
)
```

### 4. Table Extraction Configuration

**V3.x (Old):**

```python
from kreuzberg import ExtractionConfig, VisionTablesConfig

config = ExtractionConfig(
    extract_tables=True,
    vision_tables_config=VisionTablesConfig(
        detection_threshold=0.7,
    ),
)
```

**V4.0 (New):**

```python
from kreuzberg import ExtractionConfig, TableExtractionConfig

config = ExtractionConfig(
    tables=TableExtractionConfig(
        detection_threshold=0.7,
    ),
)
```

**Renamed:** `VisionTablesConfig` → `TableExtractionConfig`

### 5. Image Extraction & OCR

**V3.x (Old):**

```python
config = ExtractionConfig(
    extract_images=True,
    image_ocr_config=ImageOCRConfig(
        enabled=True,
        min_dimensions=(100, 100),
    ),
)
```

**V4.0 (New):**

```python
config = ExtractionConfig(
    images=ImageExtractionConfig(
        ocr_min_dimensions=(100, 100),
        # If ocr_min_dimensions is set, OCR is enabled
    ),
)
```

**Renamed:** `ImageOCRConfig` → `ImageExtractionConfig`

**Field Changes:**

- `enabled` removed (implied by presence of `ocr_min_dimensions`)
- `min_dimensions` → `ocr_min_dimensions`
- `max_dimensions` → `ocr_max_dimensions`
- `allowed_formats` → `ocr_allowed_formats`

### 6. Entity Extraction

**V3.x (Old):**

```python
from kreuzberg import ExtractionConfig, SpacyEntityExtractionConfig

config = ExtractionConfig(
    extract_entities=True,
    entity_extraction_config=SpacyEntityExtractionConfig(),
)
```

**V4.0 (New):**

```python
from kreuzberg import ExtractionConfig, EntityExtractionConfig

config = ExtractionConfig(
    entities=EntityExtractionConfig(),
)
```

**Renamed:** `SpacyEntityExtractionConfig` → `EntityExtractionConfig`

### 7. Keyword Extraction

**V3.x (Old):**

```python
config = ExtractionConfig(
    extract_keywords=True,
    keyword_count=10,
)
```

**V4.0 (New):**

```python
config = ExtractionConfig(
    keywords=KeywordExtractionConfig(
        top_k=10,  # Renamed from keyword_count
    ),
)
```

**Renamed:** `keyword_count` → `top_k`

## Type System Changes

### 1. msgspec.Struct Instead of Dataclasses

**V3.x (Old):**

```python
from dataclasses import dataclass

# Configs were mutable dataclasses
config = ExtractionConfig(ocr_backend="tesseract")
config.ocr_backend = "easyocr"  # Mutable
```

**V4.0 (New):**

```python
import msgspec

# Configs are frozen msgspec.Struct (immutable)
config = ExtractionConfig(ocr=TesseractConfig())
config.ocr = EasyOCRConfig()  # ERROR: Struct is frozen!
```

**Impact:**

- Configs are immutable (frozen)
- No `.to_dict()` method - use `msgspec.structs.asdict()` instead
- Better performance and memory efficiency

### 2. Collections Must Be Tuples

**V3.x (Old):**

```python
config = ExtractionConfig(
    pdf_password=["pass1", "pass2"],  # List allowed
    validators=[validator1, validator2],  # List allowed
)
```

**V4.0 (New):**

```python
config = ExtractionConfig(
    pdf_password=("pass1", "pass2"),  # Must be tuple or str
    validators=(validator1, validator2),  # Must be tuple
)
```

**All sequence parameters now require tuples for immutability.**

### 3. Method Changes

**Removed Methods:**

- `ExtractionConfig.get_config_dict()` - Use `msgspec.structs.asdict()` instead
- `ExtractionConfig.to_dict()` - Use `msgspec.structs.asdict()` instead
- Config classes no longer have `.to_dict()` methods

**Still Available:**

- `ExtractionResult.to_dict()` - Remains (dataclass with custom method)
- `ExtractionResult.export_tables_to_csv()`
- `ExtractionResult.export_tables_to_tsv()`

**Migration:**

```python
# V3
config_dict = config.to_dict()

# V4
import msgspec

config_dict = msgspec.structs.asdict(config)
```

## Class Renames

| V3 Class                      | V4 Class                 |
| ----------------------------- | ------------------------ |
| `GMFTConfig`                  | `TableExtractionConfig`  |
| `VisionTablesConfig`          | `TableExtractionConfig`  |
| `SpacyEntityExtractionConfig` | `EntityExtractionConfig` |
| `ImageOCRConfig`              | `ImageExtractionConfig`  |

## Parameter Renames

### ChunkingConfig

- No renames (new in V4)

### KeywordExtractionConfig

- `keyword_count` → `top_k`

### ImageExtractionConfig

- `enabled` → removed (implied by other fields)
- `min_dimensions` → `ocr_min_dimensions`
- `max_dimensions` → `ocr_max_dimensions`
- `allowed_formats` → `ocr_allowed_formats`

### TableExtractionConfig

- `vision_tables_config` → `tables` (in ExtractionConfig)

## API Server Changes

### Request Format Changes

**V3.x (Old):**

```bash
# Query parameters (no longer supported)
curl -X POST "http://localhost:8000/extract?chunk_content=true&max_chars=500" \
  -F "data=@document.pdf"
```

**V4.0 (New):**

```bash
# JSON config in multipart form (V4 structure required)
curl -F "files=@document.pdf" \
     -F 'config={"chunking":{"max_chars":500}}' \
     http://localhost:8000/extract

# With OCR (tagged union format)
curl -F "files=@document.pdf" \
     -F 'config={"ocr":{"backend":"tesseract","language":"eng"},"tables":{}}' \
     http://localhost:8000/extract
```

**Breaking Changes:**

- No query parameter support
- No header-based config (V3 X-Extraction-Config header removed)
- Must use `config` field in multipart form data
- Config must use V4 nested structure

## Migration Checklist

- [ ] **Update Python version to 3.10+**
- [ ] **Replace all V3 config patterns:**
    - [ ] `ocr_backend="X", ocr_config=XConfig()` → `ocr=XConfig()`
    - [ ] `extract_tables=True` → `tables=TableExtractionConfig()`
    - [ ] `extract_images=True` → `images=ImageExtractionConfig()`
    - [ ] `extract_keywords=True` → `keywords=KeywordExtractionConfig()`
    - [ ] `extract_entities=True` → `entities=EntityExtractionConfig()`
    - [ ] `chunk_content=True, max_chars=X` → `chunking=ChunkingConfig(max_chars=X)`
    - [ ] `auto_detect_language=True` → `language_detection=LanguageDetectionConfig()`
- [ ] **Update class names:**
    - [ ] `GMFTConfig` → `TableExtractionConfig`
    - [ ] `VisionTablesConfig` → `TableExtractionConfig`
    - [ ] `SpacyEntityExtractionConfig` → `EntityExtractionConfig`
    - [ ] `ImageOCRConfig` → `ImageExtractionConfig`
- [ ] **Update parameter names:**
    - [ ] `keyword_count` → `top_k`
    - [ ] `image_ocr_config.min_dimensions` → `images.ocr_min_dimensions`
- [ ] **Fix collection types:**
    - [ ] All lists → tuples
    - [ ] `pdf_password=[...]` → `pdf_password=(...)`
    - [ ] `validators=[...]` → `validators=(...)`
- [ ] **Update method calls:**
    - [ ] `config.to_dict()` → `msgspec.structs.asdict(config)`
    - [ ] `config.get_config_dict()` → `msgspec.structs.asdict(config)`
- [ ] **Update API requests:**
    - [ ] Remove query parameters
    - [ ] Use `config` in multipart form data
    - [ ] Use V4 nested config structure
- [ ] **Update TOML config files to V4 format**
- [ ] **Test all extraction functionality**

## Configuration File Migration

**V3 kreuzberg.toml:**

```toml
ocr_backend = "tesseract"
extract_tables = true
chunk_content = true
max_chars = 1000
extract_keywords = true
keyword_count = 5

[ocr_config]
language = "eng"
psm = 6

[vision_tables]
detection_threshold = 0.7
```

**V4 kreuzberg.toml:**

```toml
[ocr]
backend = "tesseract"
language = "eng"
psm = 6

[tables]
detection_threshold = 0.7

[chunking]
max_chars = 1000
max_overlap = 200

[keywords]
top_k = 5
```

## Complete Migration Example

**V3.x Code:**

```python
from kreuzberg import (
    extract_file,
    ExtractionConfig,
    TesseractConfig,
    VisionTablesConfig,
)

config = ExtractionConfig(
    ocr_backend="tesseract",
    ocr_config=TesseractConfig(language="eng"),
    extract_tables=True,
    vision_tables_config=VisionTablesConfig(detection_threshold=0.7),
    chunk_content=True,
    max_chars=1000,
    max_overlap=200,
    extract_keywords=True,
    keyword_count=5,
)

result = extract_file("document.pdf", config=config)
```

**V4.0 Code:**

```python
from kreuzberg import (
    extract_file,
    ExtractionConfig,
    TesseractConfig,
    TableExtractionConfig,
    ChunkingConfig,
    KeywordExtractionConfig,
)

config = ExtractionConfig(
    ocr=TesseractConfig(language="eng"),
    tables=TableExtractionConfig(detection_threshold=0.7),
    chunking=ChunkingConfig(max_chars=1000, max_overlap=200),
    keywords=KeywordExtractionConfig(top_k=5),
)

result = extract_file("document.pdf", config=config)
```

## Getting Help

If you encounter issues during migration:

- **Documentation**: [kreuzberg.dev](https://kreuzberg.dev)
- **GitHub Issues**: [github.com/Goldziher/kreuzberg/issues](https://github.com/Goldziher/kreuzberg/issues)
- **API Reference**: See updated type signatures in documentation

## Why These Changes?

V4's breaking changes bring significant benefits:

1. **Type Safety**: Tagged unions eliminate backend/config mismatches
1. **Performance**: msgspec.Struct is faster and more memory efficient
1. **Immutability**: Frozen configs prevent accidental mutations
1. **Clarity**: Nested objects make configuration structure obvious
1. **Validation**: Stronger compile-time and runtime type checking
1. **Maintainability**: Cleaner codebase with less legacy code
