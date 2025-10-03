# Framework Benchmarks

Comparative performance analysis of document extraction frameworks.

## Overview

The benchmark suite evaluates document processing frameworks across standardized metrics using identical test conditions. All frameworks process the same document set with consistent resource monitoring and timeout parameters.

## Tested Frameworks

| Framework    | Version | Description                             |
| ------------ | ------- | --------------------------------------- |
| Kreuzberg v4 | 4.x     | Current version with async/sync APIs    |
| Kreuzberg v3 | 3.15+   | Previous version for regression testing |
| Extractous   | 0.1+    | Rust-based with Apache Tika backend     |
| Unstructured | 0.18+   | Enterprise document processing          |
| MarkItDown   | 0.0.1+  | Microsoft markdown converter            |
| Docling      | 2.41+   | IBM Research document understanding     |

## Measurement Methodology

### Performance Metrics

- **Processing Time**: Wall-clock time from start to completion
- **Memory Usage**: Peak RSS (Resident Set Size) during extraction
- **CPU Utilization**: Average CPU percentage during processing
- **Success Rate**: Percentage of documents processed without errors
- **Throughput**: Documents per second and megabytes per second

### Test Conditions

- **Timeout**: 300 seconds per document
- **Iterations**: 3 runs per document (default)
- **Monitoring**: Resource sampling at 50ms intervals
- **Environment**: Isolated execution per framework
- **Error Handling**: Complete failure categorization

### Test Dataset

The benchmark uses the main kreuzberg test suite located at `tests/test_source_files/`:

- **Document Types**: PDF, DOCX, PPTX, HTML, images, email, text
- **Size Distribution**:
    - Tiny: \<100KB
    - Small: 100KB-1MB
    - Medium: 1MB-10MB
    - Large: >10MB
- **Language Coverage**: English, Hebrew, German, Chinese, Japanese, Korean
- **Format Complexity**: Simple text to complex layouts with tables and images

## Result Categories

### Speed Analysis

Processing time comparisons across document types and sizes.

### Memory Efficiency

Peak memory usage during document extraction.

### Reliability Metrics

Success rates and error pattern analysis.

### Format Support

Coverage analysis for different document types.

## Data Access

### Raw Results

- JSON format with complete metrics
- CSV exports for external analysis
- Per-document timing and resource data

### Aggregated Reports

- Framework comparison matrices
- Performance trend analysis
- Statistical summaries

### Visualizations

- Performance charts
- Memory usage patterns
- Success rate distributions

## Configuration Parameters

Default benchmark settings can be modified:

```python
# Timeout settings
DEFAULT_TIMEOUT = 300  # seconds

# Resource monitoring
MEMORY_SAMPLING_INTERVAL = 0.05  # 50ms

# Iteration count
DEFAULT_ITERATIONS = 3

# Quality assessment
ENABLE_QUALITY_METRICS = False
```

## Running Benchmarks

### Command Line Interface

```bash
# Run all frameworks
uv run python -m src.cli benchmark

# Specific frameworks
uv run python -m src.cli benchmark --framework kreuzberg_sync,extractous

# Document categories
uv run python -m src.cli benchmark --category tiny,small

# Generate reports
uv run python -m src.cli report --output-format html
```

### Output Formats

- **JSON**: Machine-readable results
- **CSV**: Spreadsheet-compatible data
- **HTML**: Interactive reports
- **Markdown**: Documentation-ready summaries

## Interpreting Results

### Performance Metrics

Results are presented as statistical summaries:

- Mean, median, standard deviation
- Min/max values
- 95th percentile times

### Comparison Methods

Framework comparisons use:

- Relative performance ratios
- Statistical significance testing
- Consistent test conditions
- Identical document sets

### Limitations

- Results reflect specific hardware configuration
- Network-dependent operations excluded
- OCR results may vary by system configuration
- Framework-specific optimizations not tuned
