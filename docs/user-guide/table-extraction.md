# Table Extraction

Kreuzberg provides multiple approaches for extracting tables from documents, each optimized for different use cases and accuracy requirements.

## Table Extraction Methods

### Vision-Based Table Extraction (Recommended)

**Best for:** Complex tables, high accuracy requirements, diverse document types

Uses computer vision models to detect and extract table structure with high accuracy.

```python
from kreuzberg import extract_file, ExtractionConfig, TableExtractionConfig

# Enable vision-based table extraction
config = ExtractionConfig(
    tables=TableExtractionConfig(
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

- Install with: `pip install "kreuzberg[vision-tables]"`
- Dependencies: Machine learning libraries (~1GB download)
- Optional: GPU acceleration support

### OCR-Based Table Extraction (Lightweight)

**Best for:** Simple tables, resource-constrained environments, fast processing

Uses OCR text analysis to identify and reconstruct table structures.

```python
from kreuzberg import extract_file, ExtractionConfig, TableExtractionConfig

# Enable OCR-based table extraction from TSV data
config = ExtractionConfig(
    tables=TableExtractionConfig(extract_from_ocr=True),
)

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

You can enable both vision-based and OCR-based table extraction simultaneously:

```python
config = ExtractionConfig(
    tables=TableExtractionConfig(
        extract_from_ocr=True,  # Enable OCR-based extraction
        detection_threshold=0.8,  # Vision-based detection threshold
        structure_threshold=0.6,  # Vision-based structure threshold
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
from kreuzberg import TableExtractionConfig

config = TableExtractionConfig(
    # Model selection
    detection_model="microsoft/table-transformer-detection",
    structure_model="microsoft/table-transformer-structure-recognition-v1.1-all",
    # Accuracy settings
    detection_threshold=0.7,  # Lower = more tables detected, more false positives
    structure_threshold=0.5,  # Lower = more lenient structure recognition
    # Performance settings
    detection_device="auto",  # "auto", "cpu", "cuda", "cuda:0", etc.
    structure_device="auto",  # Use same device or specify different
    # Caching
    model_cache_dir="/custom/cache/path",  # Optional custom cache location
    # Table filtering
    min_table_area=1000,  # Minimum table area in pixels²
    max_table_area=None,  # Maximum table area (None = no limit)
    crop_padding=20,  # Pixels to add around detected tables
    # Cell confidence
    cell_confidence_table=0.3,  # Confidence threshold for table cells
)
```

### OCR-Based Table Extraction

```python
from kreuzberg import ExtractionConfig, TableExtractionConfig, TesseractConfig

config = ExtractionConfig(
    tables=TableExtractionConfig(extract_from_ocr=True),
    # Optional: Configure OCR settings for better table detection
    ocr=TesseractConfig(
        output_format="tsv",  # Use TSV format for coordinate data
        enable_table_detection=True,  # Enable coordinate-based table detection
    ),
)
```

## Choosing the Right Method

### Use Vision-Based Table Extraction When

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
# Vision-based table extraction
[tables]
detection_threshold = 0.7
structure_threshold = 0.5
detection_device = "auto"
structure_device = "auto"
extract_from_ocr = false

# Enable OCR-based table extraction
[tables]
extract_from_ocr = true

# Or enable both methods
[tables]
extract_from_ocr = true
detection_threshold = 0.8
structure_threshold = 0.6

# OCR configuration for table extraction
[ocr]
backend = "tesseract"
output_format = "tsv"
enable_table_detection = true
```

### pyproject.toml

```toml
[tool.kreuzberg.tables]
detection_threshold = 0.8
structure_threshold = 0.6
extract_from_ocr = true

[tool.kreuzberg.ocr]
backend = "tesseract"
output_format = "tsv"
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
    print("\nStructured data:")
    print(table["df"].head())

    # Table metadata
    print(f"\nFound on page: {table['page_number']}")

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
**Solution:** Install with `pip install "kreuzberg[vision-tables]"`

**Problem:** Slow processing
**Solution:** Enable GPU acceleration with `detection_device="cuda"` or increase batch processing

**Problem:** No tables detected
**Solution:** Lower `detection_threshold` (try 0.5 or 0.6)

**Problem:** Poor table structure
**Solution:** Lower `structure_threshold` or adjust `cell_confidence_table`

### OCR-Based Method Issues

**Problem:** Tables not detected
**Solution:** Enable `extract_from_ocr=True` in `TableExtractionConfig`

**Problem:** Poor table structure
**Solution:** Ensure OCR is configured with `output_format="tsv"` and `enable_table_detection=True`

**Problem:** Missing text in tables
**Solution:** Improve OCR quality with higher DPI or better image preprocessing

**Problem:** False table detection
**Solution:** Use vision-based method for better accuracy or combine both methods
