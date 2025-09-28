# Table Extraction

Kreuzberg provides multiple approaches for extracting tables from documents, each optimized for different use cases and accuracy requirements.

## Table Extraction Methods

### Vision-Based Table Extraction (Recommended)

**Best for:** Complex tables, high accuracy requirements, diverse document types

Uses computer vision models to detect and extract table structure with high accuracy.

```python
from kreuzberg import extract_file, ExtractionConfig, GMFTConfig

# Enable vision-based table extraction
config = ExtractionConfig(
    extract_tables=True,
    gmft_config=GMFTConfig(
        detection_threshold=0.7,  # Confidence threshold for finding tables
        structure_threshold=0.5,  # Confidence threshold for table structure
        detection_device="auto",  # Use available GPU/CPU
        structure_device="auto",  # Use available GPU/CPU
    ),
)

result = await extract_file("document_with_tables.pdf", config=config)

# Access extracted tables
for i, table in enumerate(result.tables):
    print(f"Table {i+1} from page {table['page_number']}:")
    print(table["text"])  # Markdown representation
    # table['df'] contains structured data as Polars DataFrame
```

**Capabilities:**

- ✅ Complex table layouts with spanning cells
- ✅ Multi-level headers and nested structures
- ✅ Works on any document type (PDF, images, presentations)
- ✅ High accuracy even with irregular table formatting
- ✅ GPU acceleration support for faster processing

**Requirements:**

- Install with: `pip install "kreuzberg[gmft]"`
- Dependencies: Machine learning libraries (~1GB download)
- Optional: GPU acceleration support

### OCR-Based Table Extraction (Lightweight)

**Best for:** Simple tables, resource-constrained environments, fast processing

Uses OCR text analysis to identify and reconstruct table structures.

```python
from kreuzberg import extract_file, ExtractionConfig

# Enable OCR-based table extraction from TSV data
config = ExtractionConfig(extract_tables_from_ocr=True)

result = await extract_file("scanned_table.pdf", config=config)

# Tables are available in the same format
for table in result.tables:
    print(table["text"])  # Markdown representation
```

**Capabilities:**

- ✅ Fast processing with minimal resource usage
- ✅ No additional dependencies beyond Tesseract
- ✅ Good for well-formatted, simple tables
- ✅ Integrates with OCR workflow

**Limitations:**

- ❌ Limited to basic row/column structures
- ❌ Cannot handle spanning cells or complex layouts
- ❌ Accuracy depends on OCR text quality

### Combining Multiple Methods

You can enable multiple table extraction methods simultaneously for comprehensive coverage:

```python
config = ExtractionConfig(
    extract_tables=True,  # Vision-based method
    extract_tables_from_ocr=True,  # OCR-based method
    gmft_config=GMFTConfig(
        detection_threshold=0.8,  # Higher threshold for vision method
    ),
)

result = await extract_file("complex_document.pdf", config=config)

# All detected tables from both methods
print(f"Found {len(result.tables)} tables total")
```

**Benefits of combining:**

- Maximum table coverage
- Vision method catches complex tables
- OCR method catches simple tables that vision might miss
- Redundancy for critical table extraction workflows

## Configuration Options

### Vision-Based Table Extraction

```python
from kreuzberg import GMFTConfig

config = GMFTConfig(
    # Model selection (optional, uses optimized defaults)
    # Accuracy settings
    detection_threshold=0.7,  # Lower = more tables detected, more false positives
    structure_threshold=0.5,  # Lower = more lenient structure recognition
    # Performance settings
    detection_device="auto",  # "auto", "cpu", "cuda", "mps"
    structure_device="auto",  # Use same device or specify different
    batch_size=1,  # Images processed per batch (GPU only)
    mixed_precision=False,  # Use FP16 for faster GPU processing
    # Caching
    model_cache_dir="/custom/cache/path",  # Optional custom cache location
    enable_model_caching=True,  # Cache models for faster subsequent runs
    # Output
    verbosity=1,  # 0=silent, 1=info, 2=debug
)
```

### OCR-Based Table Extraction

```python
from kreuzberg import ExtractionConfig, TesseractConfig

config = ExtractionConfig(
    extract_tables_from_ocr=True,
    # Optional: Configure OCR settings for better table detection
    ocr_config=TesseractConfig(
        output_format="tsv",  # Use TSV format for coordinate data
        enable_table_detection=True,  # Enable HOCR-based table detection
        table_column_threshold=20,  # Pixel threshold for column detection
        table_row_threshold_ratio=0.5,  # Row detection sensitivity
        table_min_confidence=30.0,  # Minimum OCR confidence
    ),
)
```

## Choosing the Right Method

### Use AI-Powered Table Extraction When

- **Document Types:** Scientific papers, financial reports, complex layouts
- **Table Complexity:** Spanning cells, multi-level headers, irregular structures
- **Accuracy Priority:** Need highest possible table extraction accuracy
- **Resources Available:** Have GPU or sufficient CPU power and storage
- **Processing Volume:** Moderate volume where quality matters more than speed

### Use OCR-Based Table Extraction When

- **Document Types:** Scanned documents, simple forms, basic spreadsheets
- **Table Complexity:** Simple row/column structures with clear borders
- **Resource Constraints:** Limited memory, storage, or processing power
- **Speed Priority:** Need fast processing with minimal overhead
- **Processing Volume:** High volume processing where speed matters

### Use Both Methods When

- **Maximum Coverage:** Need to extract every possible table
- **Mixed Document Types:** Processing diverse documents with varying table complexity
- **Quality Assurance:** Want redundancy for critical table extraction workflows
- **Unknown Table Types:** Unsure about table complexity in your document set

## Performance Considerations

### Vision-Based Method

- **GPU Acceleration:** 3-5x faster with CUDA or Apple Silicon
- **Model Caching:** First run downloads ~1GB models, subsequent runs are fast
- **Memory Usage:** ~2-4GB RAM during processing
- **Processing Time:** 2-10 seconds per page with tables (depending on hardware)

### OCR-Based Method

- **Lightweight:** ~100MB additional memory usage
- **Fast Processing:** \<1 second per page typically
- **Scalable:** Linear scaling with document size
- **No Downloads:** Uses existing Tesseract installation

## Configuration Files

### kreuzberg.toml

```toml
# Enable both table extraction methods
extract_tables = true
extract_tables_from_ocr = true

# Vision-based table extraction settings
[gmft]
detection_threshold = 0.7
structure_threshold = 0.5
detection_device = "auto"
structure_device = "auto"
enable_model_caching = true
verbosity = 1

# OCR-based table extraction settings
[tesseract]
output_format = "tsv"
enable_table_detection = true
table_column_threshold = 20
table_min_confidence = 30.0
```

### pyproject.toml

```toml
[tool.kreuzberg]
extract_tables = true
extract_tables_from_ocr = true

[tool.kreuzberg.gmft]
detection_threshold = 0.8
structure_threshold = 0.6

[tool.kreuzberg.tesseract]
enable_table_detection = true
```

## Working with Extracted Tables

```python
# Access table data in multiple formats
for table in result.tables:
    # Markdown text representation
    print("Markdown:")
    print(table["text"])

    # Structured data as Polars DataFrame
    print("\\nStructured data:")
    print(table["df"].head())

    # Table metadata
    print(f"\\nFound on page: {table['page_number']}")

    # Save table image (vision method only)
    if "cropped_image" in table:
        table["cropped_image"].save(f"table_{table['page_number']}.png")

    # Convert to other formats
    table_csv = table["df"].write_csv()
    table_json = table["df"].write_json()
```

## Troubleshooting

### Vision-Based Method Issues

**Problem:** "Missing dependency" error
**Solution:** Install with `pip install "kreuzberg[gmft]"`

**Problem:** Slow processing
**Solution:** Enable GPU acceleration or increase `batch_size` for GPU processing

**Problem:** No tables detected
**Solution:** Lower `detection_threshold` (try 0.5 or 0.6)

**Problem:** Poor table structure
**Solution:** Lower `structure_threshold` or try different model variant

### OCR-Based Method Issues

**Problem:** Tables not detected
**Solution:** Enable `enable_table_detection=True` in OCR config

**Problem:** Poor table structure
**Solution:** Adjust `table_column_threshold` or `table_row_threshold_ratio`

**Problem:** Missing text in tables
**Solution:** Lower `table_min_confidence` threshold

**Problem:** False table detection
**Solution:** Increase confidence thresholds or use vision method for better accuracy
