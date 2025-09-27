# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [4.0.0] - Unreleased

### Breaking Changes

#### GMFT Table Extraction Configuration

The GMFT table extraction configuration has been completely redesigned for better control and to use the latest TATR v1.1 models:

**Old Configuration (v3.x):**

```python
from kreuzberg._types import GMFTConfig

config = GMFTConfig(detector_base_threshold=0.5, formatter_base_threshold=0.7, verbosity=1)
```

**New Configuration (v4.0):**

```python
from kreuzberg._types import GMFTConfig

config = GMFTConfig(
    # Model paths - now using TATR v1.1
    detection_model="microsoft/table-transformer-detection",
    structure_model="microsoft/table-transformer-structure-recognition-v1.1-all",
    # Model cache directory (optional)
    model_cache_dir="/path/to/cache",
    # Detection settings
    detection_threshold=0.7,
    detection_device="auto",  # "auto", "cpu", "cuda", "mps"
    # Structure recognition settings
    structure_threshold=0.5,
    structure_device="auto",
    # Additional options
    enable_model_caching=True,
    batch_size=1,
    mixed_precision=False,
    verbosity=1,
)
```

#### Type System Changes

- `TablePredictionsDict` renamed to `TablePredictions`
- All prediction types are now hashable frozen dataclasses for better caching support
- `BboxPredictions` is now a frozen dataclass instead of TypedDict

#### Removed Classes

The following internal configuration classes have been removed and replaced by `GMFTConfig`:

- `TableDetectorConfig`
- `TableFormatConfig`

#### Model Updates

Default models updated from TATR v1.0 to TATR v1.1:

- Detection model remains: `microsoft/table-transformer-detection`
- Structure model updated to: `microsoft/table-transformer-structure-recognition-v1.1-all`

Three structure model variants are now available:

- `v1.1-all`: Best overall performance (default)
- `v1.1-pub`: Optimized for published/printed tables
- `v1.1-fin`: Optimized for financial tables

### Added

- **LRU Caching**: Strategic caching added to expensive operations for better performance
- **Model Cache Directory**: Configure where HuggingFace models are cached
- **Device Selection**: Separate device configuration for detection and structure models
- **TATR v1.1 Support**: Support for latest Table Transformer models
- **Batch Processing**: Optional batch_size configuration for processing multiple images
- **Mixed Precision**: Optional mixed precision inference for better GPU performance

### Changed

- Python 3.10+ only - using modern union syntax (`|` instead of `Union`)
- Functional programming patterns preferred over classes where appropriate
- All configuration dataclasses are now hashable and frozen

### Fixed

- Improved test coverage from 66% to 76%
- Fixed type checking errors with TypedDict to dataclass conversion
- Better error handling for missing ML dependencies
