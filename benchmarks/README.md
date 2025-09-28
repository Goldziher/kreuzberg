# Document Extraction Framework Benchmarks

Performance testing suite for Python document processing frameworks.

## Overview

This benchmark suite evaluates document extraction frameworks across multiple metrics:

- **Performance**: Processing speed, memory usage, CPU utilization
- **Reliability**: Success rates, error handling, timeout behavior
- **Quality**: Text extraction accuracy and completeness
- **Coverage**: File format support

## Tested Frameworks

- **Kreuzberg v4** (sync/async) - Multi-format extraction with OCR backends
- **Kreuzberg v3** - Previous version for comparison
- **Extractous** - Rust-based with Apache Tika integration
- **Unstructured** - Enterprise document processing
- **MarkItDown** - Microsoft's markdown converter
- **Docling** - IBM Research document understanding

## Quick Start

```bash
# Install all dependencies
uv sync --all-extras --all-packages --all-groups

# Run benchmarks
uv run python -m src.cli benchmark

# Generate reports
uv run python -m src.cli report --output-format html
```

## CLI Commands

```bash
# List available options
uv run python -m src.cli --help

# Run specific frameworks
uv run python -m src.cli benchmark --framework kreuzberg_sync,extractous

# Test document categories
uv run python -m src.cli benchmark --category tiny,small,medium

# Generate visualizations
uv run python -m src.cli visualize
```

## Test Dataset

The benchmark uses test files from the main kreuzberg repository located at `../tests/test_source_files/`:

- **File Types**: PDF, DOCX, HTML, images, email, text formats
- **Size Categories**: Tiny (\<100KB), Small (100KB-1MB), Medium (1MB-10MB), Large (10MB+)
- **Languages**: English, Hebrew, German, Chinese, Japanese, Korean

## Methodology

- **Isolation**: Each framework tested independently
- **Metrics Collection**: Processing time, memory usage, success rates
- **Timeout Handling**: 300 seconds per file
- **Error Tracking**: Comprehensive failure analysis
- **Reproducibility**: Deterministic test execution

## Configuration

Test parameters can be adjusted in `src/types.py`:

- Timeout values
- Memory limits
- Quality assessment settings
- Framework-specific configurations

## Output

Results are stored in:

- `results/` - Raw benchmark data (JSON/CSV)
- `reports/` - Generated analysis reports
- `visualizations/` - Charts and graphs

## Development

```bash
# Install development dependencies
uv sync --all-extras --all-packages --all-groups

# Run tests
uv run pytest tests/

# Format code
ruff format && ruff check
```
