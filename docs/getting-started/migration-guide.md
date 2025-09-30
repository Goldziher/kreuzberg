# Migration Guide: v3.x to v4.0

This guide helps you migrate from Kreuzberg v3.x to v4.0, which introduces breaking changes with deprecated parameter removal and architectural improvements.

## Overview of Breaking Changes

Version 4.0 removes all previously deprecated configuration parameters and introduces a hybrid Rust-Python architecture for improved performance. The main changes affect:

- OCR backend configuration (EasyOCR, PaddleOCR)
- GMFT table extraction configuration
- Image OCR configuration in ExtractionConfig
- Python 3.10+ requirement (using modern union syntax)

## Deprecated Parameter Removal

### EasyOCRConfig

**Removed Parameter:**

- `use_gpu` → Use `device="cuda"` or `device="auto"` instead

**Migration:**

```python
# v3.x (deprecated)
from kreuzberg import extract_file, ExtractionConfig, EasyOCRConfig

config = ExtractionConfig(
    ocr_backend="easyocr",
    ocr_config=EasyOCRConfig(
        language_list=["en", "de"],
        use_gpu=True,  # Deprecated
    ),
)

# v4.0 (current)
from kreuzberg import extract_file, ExtractionConfig, EasyOCRConfig

config = ExtractionConfig(
    ocr_backend="easyocr",
    ocr_config=EasyOCRConfig(
        language_list=["en", "de"],
        device="auto",  # Auto-detect CUDA/MPS/CPU
    ),
)
```

**Device Options:**

- `"auto"` - Automatically detect and use best available device (CUDA > MPS > CPU)
- `"cuda"` - Use NVIDIA GPU (requires CUDA-enabled PyTorch)
- `"mps"` - Use Apple Silicon GPU (macOS only)
- `"cpu"` - Use CPU only

### PaddleOCRConfig

**Removed Parameters:**

- `use_gpu` → Use `device="cuda"` or `device="auto"` instead
- `gpu_mem` → No longer supported (parameter unused in PaddleOCR 3.2.0+)
- `gpu_memory_limit` → No longer supported (parameter unused in PaddleOCR 3.2.0+)
- `use_angle_cls` → Use `use_textline_orientation` instead
- `det_db_box_thresh` → Use `text_det_box_thresh` instead
- `det_db_thresh` → Use `text_det_thresh` instead
- `det_db_unclip_ratio` → Use `text_det_unclip_ratio` instead

**Migration:**

```python
# v3.x (deprecated)
from kreuzberg import extract_file, ExtractionConfig, PaddleOCRConfig

config = ExtractionConfig(
    ocr_backend="paddleocr",
    ocr_config=PaddleOCRConfig(
        language="ch",
        use_gpu=True,  # Deprecated
        gpu_mem=4000,  # Deprecated
        use_angle_cls=True,  # Deprecated
        det_db_box_thresh=0.6,  # Deprecated
        det_db_thresh=0.3,  # Deprecated
        det_db_unclip_ratio=2.0,  # Deprecated
    ),
)

# v4.0 (current)
from kreuzberg import extract_file, ExtractionConfig, PaddleOCRConfig

config = ExtractionConfig(
    ocr_backend="paddleocr",
    ocr_config=PaddleOCRConfig(
        language="ch",
        device="auto",  # Replaces use_gpu
        use_textline_orientation=True,  # Replaces use_angle_cls
        text_det_box_thresh=0.6,  # Replaces det_db_box_thresh
        text_det_thresh=0.3,  # Replaces det_db_thresh
        text_det_unclip_ratio=2.0,  # Replaces det_db_unclip_ratio
    ),
)
```

**Note:** PaddleOCR does not support MPS (Apple Silicon). The `device` parameter only accepts `"auto"`, `"cuda"`, or `"cpu"`.

### GMFTConfig

**Removed Parameter:**

- `low_memory` → Use `model="lite"` instead

**Migration:**

```python
# v3.x (deprecated)
from kreuzberg import extract_file, ExtractionConfig, GMFTConfig

config = ExtractionConfig(
    extract_tables=True,
    gmft_config=GMFTConfig(
        low_memory=True,  # Deprecated
    ),
)

# v4.0 (current)
from kreuzberg import extract_file, ExtractionConfig, GMFTConfig

config = ExtractionConfig(
    extract_tables=True,
    gmft_config=GMFTConfig(
        model="lite",  # Explicitly specify lite model
    ),
)
```

**Model Options:**

- `"standard"` - Full-size model with best accuracy (default)
- `"lite"` - Smaller model for memory-constrained environments
- `"auto"` - Automatically choose based on available memory

### ExtractionConfig (Image OCR)

**Removed Parameters:**

- `ocr_extracted_images` → Use `image_ocr_config=ImageOCRConfig(enabled=True)` instead
- `image_ocr_backend` → Use `image_ocr_config=ImageOCRConfig(backend="tesseract")` instead
- `image_ocr_min_dimensions` → Use `image_ocr_config=ImageOCRConfig(min_dimensions=(50, 50))` instead
- `image_ocr_max_dimensions` → Use `image_ocr_config=ImageOCRConfig(max_dimensions=(10000, 10000))` instead
- `image_ocr_formats` → Use `image_ocr_config=ImageOCRConfig(allowed_formats=frozenset(...))` instead

**Migration:**

```python
# v3.x (deprecated)
from kreuzberg import extract_file, ExtractionConfig

config = ExtractionConfig(
    extract_images=True,
    ocr_extracted_images=True,  # Deprecated
    image_ocr_backend="tesseract",  # Deprecated
    image_ocr_min_dimensions=(100, 100),  # Deprecated
    image_ocr_max_dimensions=(5000, 5000),  # Deprecated
)

# v4.0 (current)
from kreuzberg import extract_file, ExtractionConfig, ImageOCRConfig

config = ExtractionConfig(
    extract_images=True,
    image_ocr_config=ImageOCRConfig(
        enabled=True,
        backend="tesseract",
        min_dimensions=(100, 100),
        max_dimensions=(5000, 5000),
    ),
)
```

**Advanced Image OCR with Custom OCR Config:**

```python
# v4.0 with custom OCR configuration
from kreuzberg import extract_file, ExtractionConfig, ImageOCRConfig, TesseractConfig

tesseract_config = TesseractConfig(
    language="eng+deu",
    psm=6,
    output_format="text",
)

config = ExtractionConfig(
    extract_images=True,
    image_ocr_config=ImageOCRConfig(
        enabled=True,
        backend="tesseract",
        ocr_config=tesseract_config,
        min_dimensions=(200, 200),
        max_dimensions=(4000, 4000),
    ),
)
```

## GMFT Configuration Redesign

Version 4.0 completely redesigns GMFT configuration to use TATR v1.1 models with simplified options.

### Old GMFT Configuration (v3.x)

```python
from kreuzberg import ExtractionConfig, GMFTConfig

config = ExtractionConfig(
    extract_tables=True,
    gmft_config=GMFTConfig(
        detector_base_threshold=0.5,
        formatter_base_threshold=0.7,
        verbosity=1,
        # Many internal parameters...
    ),
)
```

### New GMFT Configuration (v4.0)

```python
from kreuzberg import ExtractionConfig, GMFTConfig

config = ExtractionConfig(
    extract_tables=True,
    gmft_config=GMFTConfig(
        # Model selection
        detection_model="microsoft/table-transformer-detection",
        structure_model="microsoft/table-transformer-structure-recognition-v1.1-all",
        # Simple thresholds
        detection_threshold=0.7,
        structure_threshold=0.5,
        # Device selection
        detection_device="auto",
        structure_device="auto",
        # Optional features
        model_cache_dir="/custom/cache/path",
        enable_model_caching=True,
        batch_size=1,
        mixed_precision=False,
        verbosity=1,
    ),
)
```

**Removed Internal Options:**

- `formatter_base_threshold`, `cell_required_confidence`
- `remove_null_rows`, `enable_multi_header`
- `semantic_spanning_cells`, `semantic_hierarchical_left_fill`
- `large_table_*` parameters
- Complex internal tuning parameters

## Architecture Changes

### Hybrid Rust-Python

Version 4.0 introduces Rust implementations for performance-critical operations:

- **XML Parsing**: Streaming Rust parser for multi-GB files
- **Plain Text/Markdown**: Rust streaming parser with metadata extraction
- **Excel Extraction**: Native Rust using Calamine (~3x speed improvement)
- **PPTX Extraction**: Complete Rust rewrite
- **Email Parsing**: Full MSG support with Rust
- **Image Preprocessing**: 2.6x speedup
- **Token Reduction**: 5-10x faster

**Impact on Users:** These changes are transparent - the Python API remains the same.

### Python 3.10+ Requirement

Version 4.0 requires Python 3.10 or higher and uses modern union syntax:

```python
# v3.x
from typing import Union, Optional

config: Optional[Union[str, int]] = None

# v4.0
config: str | int | None = None
```

### Build System Migration

Version 4.0 migrates from Hatchling to Maturin for Rust-Python integration:

- **For Users:** No change - install with `pip install kreuzberg` as before
- **For Contributors:** Use `maturin develop` instead of `pip install -e .`

## Dependency Changes

### Removed Python Dependencies

- `python-pptx` (replaced by Rust implementation)
- `python-calamine` (replaced by Rust implementation)
- `chardetng-py` (replaced by Rust implementation)

### Updated Requirements

- **GMFT**: Now requires `torch>=2.8.0` and `transformers>=4.35.2`
- **Build**: Requires `maturin>=1.9.0` for development builds

## Migration Checklist

Use this checklist to ensure a smooth migration:

- [ ] Update Python version to 3.10 or higher
- [ ] Replace `use_gpu` with `device` in EasyOCRConfig and PaddleOCRConfig
- [ ] Update PaddleOCR threshold parameters (`det_db_*` → `text_det_*`)
- [ ] Replace `use_angle_cls` with `use_textline_orientation` in PaddleOCRConfig
- [ ] Replace `low_memory` with `model` in GMFTConfig
- [ ] Migrate flat image OCR parameters to nested `ImageOCRConfig`
- [ ] Update GMFT configuration to use simplified v4.0 options
- [ ] Test your application with the new configuration
- [ ] Update any API server query parameters to use nested structures

## Getting Help

If you encounter issues during migration:

- **Documentation**: [kreuzberg.dev](https://kreuzberg.dev)
- **GitHub Issues**: [github.com/Goldziher/kreuzberg/issues](https://github.com/Goldziher/kreuzberg/issues)
- **Examples**: See `docs/examples/` for updated code samples

## API Server Migration

If using the REST API server, update your requests:

### Old Query Parameters (v3.x)

```bash
curl -X POST "http://localhost:8000/extract?ocr_extracted_images=true&image_ocr_backend=tesseract" \
  -F "data=@document.pdf"
```

### New Header-Based Configuration (v4.0)

```bash
curl -X POST http://localhost:8000/extract \
  -H "X-Extraction-Config: {\"extract_images\": true, \"image_ocr_config\": {\"enabled\": true, \"backend\": \"tesseract\"}}" \
  -F "data=@document.pdf"
```

The flat query parameters are no longer supported for image OCR configuration. Use the `X-Extraction-Config` header with nested `image_ocr_config` structure instead.
