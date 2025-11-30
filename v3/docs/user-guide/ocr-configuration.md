# OCR Configuration

Kreuzberg offers simple configuration options for OCR to extract text from images and scanned documents.

## OCR Configuration

All extraction functions in Kreuzberg accept an [`ExtractionConfig`](../api-reference/types.md#extractionconfig) object that can contain OCR configuration:

### Language Configuration

The `language` parameter in a [`TesseractConfig`](../api-reference/ocr-configuration.md#tesseractconfig) object specifies which language model Tesseract should use for OCR:

```python
from kreuzberg import extract_file, ExtractionConfig, TesseractConfig

# Extract text from a German document
result = await extract_file("german_document.pdf", config=ExtractionConfig(ocr_config=TesseractConfig(language="deu")))
```

#### Supported Language Codes

| Language            | Code      | Language           | Code      |
| ------------------- | --------- | ------------------ | --------- |
| English             | `eng`     | German             | `deu`     |
| French              | `fra`     | Spanish            | `spa`     |
| Italian             | `ita`     | Japanese           | `jpn`     |
| Korean              | `kor`     | Simplified Chinese | `chi_sim` |
| Traditional Chinese | `chi_tra` | Russian            | `rus`     |
| Arabic              | `ara`     | Hindi              | `hin`     |

#### Multi-Language Support

You can specify multiple languages by joining codes with a plus sign:

```python
# Document contains both English and German text
result = await extract_file("multilingual.pdf", config=ExtractionConfig(ocr_config=TesseractConfig(language="eng+deu")))
```

!!! note

    The order of languages affects processing time and accuracy. The first language is treated as the primary language.

#### Language Installation

For Tesseract to recognize languages other than English, you need to install the corresponding language data:

- **Ubuntu/Debian**: `sudo apt-get install tesseract-ocr-<lang-code>`
- **macOS**: `brew install tesseract-lang` (installs all languages)
- **Windows**: Download language data from [GitHub](https://github.com/tesseract-ocr/tessdata)

### Page Segmentation Mode (PSM)

The `psm` parameter in a [`TesseractConfig`](../api-reference/ocr-configuration.md#tesseractconfig) object controls how Tesseract analyzes the layout of the page:

```python
from kreuzberg import extract_file, ExtractionConfig, TesseractConfig, PSMMode

# Extract text from a document with a simple layout
result = await extract_file("document.pdf", config=ExtractionConfig(ocr_config=TesseractConfig(psm=PSMMode.SINGLE_BLOCK)))
```

#### Available PSM Modes

| Mode          | Enum Value              | Description                                              | Best For                                       |
| ------------- | ----------------------- | -------------------------------------------------------- | ---------------------------------------------- |
| Auto Only     | `PSMMode.AUTO_ONLY`     | Automatic segmentation without orientation detection     | Modern documents (default - fastest)           |
| Automatic     | `PSMMode.AUTO`          | Automatic page segmentation with orientation detection   | Rotated/skewed documents                       |
| Single Block  | `PSMMode.SINGLE_BLOCK`  | Treat the image as a single text block                   | Simple layouts, preserving paragraph structure |
| Single Column | `PSMMode.SINGLE_COLUMN` | Assume a single column of text                           | Books, articles, single-column documents       |
| Single Line   | `PSMMode.SINGLE_LINE`   | Treat the image as a single text line                    | Receipts, labels, single-line text             |
| Single Word   | `PSMMode.SINGLE_WORD`   | Treat the image as a single word                         | Word recognition tasks                         |
| Sparse Text   | `PSMMode.SPARSE_TEXT`   | Find as much text as possible without assuming structure | Forms, tables, scattered text                  |

### Forcing OCR

By default, Kreuzberg will only use OCR for images and scanned PDFs. For searchable PDFs, it will extract text directly. You can override this behavior with the `force_ocr` parameter in the `ExtractionConfig` object:

```python
from kreuzberg import extract_file, ExtractionConfig

# Force OCR even for searchable PDFs
result = await extract_file("searchable.pdf", config=ExtractionConfig(force_ocr=True))
```

This is useful when:

- The PDF contains both searchable text and images with text
- The embedded text in the PDF has encoding or extraction issues
- You want consistent processing across all documents

## OCR Engine Selection

Kreuzberg supports multiple OCR engines:

### Tesseract (Default)

Tesseract is the default OCR engine and requires no additional installation beyond the system dependency.

### EasyOCR (Optional)

To use EasyOCR:

1. Install with the extra: `pip install "kreuzberg[easyocr]"`
1. Use the `ocr_backend` parameter in the `ExtractionConfig` object:

```python
from kreuzberg import extract_file, ExtractionConfig, EasyOCRConfig  # EasyOCRConfig is imported from kreuzberg

result = await extract_file(
    "document.jpg",
    config=ExtractionConfig(ocr_backend="easyocr", ocr_config=EasyOCRConfig(language_list=["en"])),
)
```

### PaddleOCR (Optional)

To use PaddleOCR:

1. Install with the extra: `pip install "kreuzberg[paddleocr]"`
1. Use the `ocr_backend` parameter in the `ExtractionConfig` object:

```python
from kreuzberg import extract_file, ExtractionConfig, PaddleOCRConfig  # PaddleOCRConfig is imported from kreuzberg

result = await extract_file(
    "document.jpg",
    config=ExtractionConfig(
        ocr_backend="paddleocr", ocr_config=PaddleOCRConfig(language="en")  # PaddleOCR uses different language codes
    ),
)
```

!!! note

    For PaddleOCR, the supported language codes are different: `ch` (Chinese), `en` (English), `french`, `german`, `japan`, and `korean`.

## Output Formats

Tesseract in Kreuzberg supports multiple output formats, each optimized for different use cases.

### Default: Markdown Format

Since v3.5.0, **markdown is the default output format** for Tesseract OCR. This provides:

- Better document structure preservation
- Readable formatting with headings, lists, and emphasis
- Clean output suitable for LLMs and downstream processing

```python
from kreuzberg import extract_file

# Uses markdown format by default
result = await extract_file("document.jpg")
print(result.content)  # Markdown-formatted text
```

### Performance Considerations

Output formats listed by speed (fastest to slowest):

#### 1. Text Format (Fastest)

Direct text extraction with minimal overhead.

```python
from kreuzberg import extract_file, ExtractionConfig, TesseractConfig

result = await extract_file("document.jpg", config=ExtractionConfig(ocr_config=TesseractConfig(output_format="text")))
```

Use when: You only need plain text without formatting.

#### 2. hOCR Format

Raw HTML-based OCR output with no post-processing.

```python
result = await extract_file("document.jpg", config=ExtractionConfig(ocr_config=TesseractConfig(output_format="hocr")))
```

Use when: You need word positions and bounding boxes for layout analysis.

#### 3. Markdown Format (Default)

Structured markdown with HTML parsing and conversion.

```python
result = await extract_file("document.jpg", config=ExtractionConfig(ocr_config=TesseractConfig(output_format="markdown")))
```

Use when: You want readable, structured output with preserved formatting.

#### 4. TSV Format

Tab-separated values with optional table detection.

```python
result = await extract_file("document.jpg", config=ExtractionConfig(ocr_config=TesseractConfig(output_format="tsv")))
```

Use when: You need confidence scores or want to extract tables.

### Table Extraction from Scanned Documents

Enable TSV-based table detection for extracting tables from scanned documents or images:

```python
from kreuzberg import extract_file, ExtractionConfig, TesseractConfig

result = await extract_file(
    "scanned_invoice.pdf",
    config=ExtractionConfig(
        ocr_config=TesseractConfig(
            output_format="tsv",
            enable_table_detection=True,
            table_column_threshold=20,  # Pixel threshold for column clustering
            table_row_threshold_ratio=0.5,  # Row threshold as ratio of text height
            table_min_confidence=30.0,  # Minimum OCR confidence for words
        )
    ),
)

# Access extracted tables
for i, table in enumerate(result.tables):
    print(f"Table {i+1}:")
    print(table["text"])  # Markdown-formatted table
    print(f"Found on page: {table['page_number']}")
```

#### Table Detection Parameters

- **`enable_table_detection`**: Set to `True` to activate table extraction
- **`table_column_threshold`**: Pixel distance for grouping words into columns (default: 20)
- **`table_row_threshold_ratio`**: Ratio of mean text height for row grouping (default: 0.5)
- **`table_min_confidence`**: Minimum OCR confidence to include words (default: 30.0)

#### Example: Processing Scanned Receipts

```python
from kreuzberg import extract_file, ExtractionConfig, TesseractConfig

# Configure for receipt processing
config = ExtractionConfig(
    ocr_config=TesseractConfig(
        output_format="tsv",
        enable_table_detection=True,
        table_column_threshold=30,  # Wider threshold for receipt columns
        psm=PSMMode.SPARSE_TEXT,  # Better for scattered text
    )
)

result = await extract_file("receipt.jpg", config=config)

# Process extracted tables
if result.tables:
    receipt_table = result.tables[0]
    print("Receipt items:")
    print(receipt_table["text"])
```

## Performance Optimization

### Default Configuration

Kreuzberg's defaults are optimized out-of-the-box for modern PDFs and standard documents:

- **PSM Mode**: `AUTO_ONLY` - Faster than `AUTO` without orientation detection overhead
- **Language Model**: Disabled by default for optimal performance on modern documents
- **Dictionary Correction**: Enabled for accuracy

The default configuration provides excellent extraction quality for:

- Modern PDFs with embedded text
- Scanned documents with clear printing
- Office documents (DOCX, PPTX, XLSX)
- Standard business documents

### Speed vs Quality Trade-offs

```python
from kreuzberg import ExtractionConfig, TesseractConfig, PSMMode

# Default configuration (optimized for modern documents)
default_config = ExtractionConfig()  # Already optimized for speed and quality

# Maximum speed configuration
speed_config = ExtractionConfig(
    ocr_backend="tesseract",
    ocr_config=TesseractConfig(
        psm=PSMMode.SINGLE_BLOCK,  # Assume simple layout
        tessedit_enable_dict_correction=False,  # Skip dictionary correction
    ),
)

# Maximum accuracy configuration (for degraded/historical documents)
accuracy_config = ExtractionConfig(
    ocr_backend="tesseract",
    ocr_config=TesseractConfig(
        psm=PSMMode.AUTO,  # Full analysis with orientation detection
        language_model_ngram_on=True,  # Enable for degraded/historical text
        tessedit_enable_dict_correction=True,  # Correct OCR errors
    ),
)
```

### Language Model N-gram Settings

The `language_model_ngram_on` parameter controls Tesseract's use of n-gram language models:

- **Default (False)**: Optimized for modern documents with clear text
- **When to enable**: Historical documents, degraded scans, handwritten text, or noisy images

```python
# For degraded or historical documents
historical_config = ExtractionConfig(
    ocr_backend="tesseract",
    ocr_config=TesseractConfig(
        language_model_ngram_on=True,  # Enable for better accuracy on poor quality text
    ),
)
```

### When to Disable OCR

For documents with text layers (searchable PDFs, Office docs), disable OCR entirely:

```python
# No OCR overhead for text documents
text_config = ExtractionConfig(ocr_backend=None)
```

This provides significant speedup (78% of PDFs have text layers and extract in \<0.01s)

## Image Processing and DPI Configuration

Kreuzberg automatically handles image size optimization to prevent OCR failures while maintaining quality. The DPI configuration system ensures optimal processing regardless of document size.

### Automatic DPI Management

By default, Kreuzberg automatically adjusts image resolution to prevent "Image too large" errors:

```python
from kreuzberg import extract_file, ExtractionConfig

# Default configuration with automatic DPI adjustment
result = await extract_file("large_document.pdf")  # Handles oversized pages automatically
```

### Custom DPI Configuration

For specific use cases, you can customize DPI settings:

```python
from kreuzberg import extract_file, ExtractionConfig

# Custom DPI settings for high-quality processing
config = ExtractionConfig(
    target_dpi=200,  # Higher DPI for better OCR accuracy
    max_image_dimension=30000,  # Allow larger images before scaling
    auto_adjust_dpi=True,  # Still auto-adjust if too large
    min_dpi=100,  # Higher minimum DPI
    max_dpi=400,  # Lower maximum DPI to prevent memory issues
)

result = await extract_file("technical_drawing.pdf", config=config)
```

### DPI Configuration Guidelines

- **target_dpi (150)**: Optimal balance between quality and performance
- **max_image_dimension (25000)**: Prevents memory exhaustion on large documents
- **auto_adjust_dpi (True)**: Automatically scales down oversized images
- **min_dpi (72)**: Minimum resolution for readable text
- **max_dpi (600)**: Maximum resolution before diminishing returns

### Performance vs Quality Trade-offs

```python
# Speed-optimized configuration
speed_config = ExtractionConfig(
    target_dpi=100,  # Lower DPI for faster processing
    max_image_dimension=15000,  # Smaller images for speed
)

# Quality-optimized configuration
quality_config = ExtractionConfig(
    target_dpi=300,  # Higher DPI for better accuracy
    max_image_dimension=40000,  # Allow larger images for detail preservation
    min_dpi=150,  # Higher minimum for small text
)
```

## Best Practices

- **Language Selection**: Always specify the correct language for your documents to improve OCR accuracy
- **PSM Mode Selection**: Choose the appropriate PSM mode based on your document layout:
    - Use `PSMMode.AUTO_ONLY` (default) for modern, well-formatted documents
    - Use `PSMMode.SINGLE_BLOCK` for simple layouts with faster processing
    - Use `PSMMode.SPARSE_TEXT` for forms or documents with tables
    - Use `PSMMode.AUTO` only when orientation detection is needed
- **Performance Optimization**:
    - Disable OCR (`ocr_backend=None`) for documents with text layers
    - Disable language model for clean documents (`language_model_ngram_on=False`)
    - Disable dictionary correction for technical documents
- **Image Quality**: For best results, ensure images are:
    - High resolution (at least 300 DPI recommended, 150 DPI minimum)
    - Well-lit with good contrast
    - Not skewed or rotated (unless using `PSMMode.AUTO`)
- **DPI Configuration**:
    - Use default settings for most documents (automatically optimized)
    - Increase `target_dpi` for documents with small text or fine details
    - Decrease `target_dpi` for faster processing of simple documents
    - Leave `auto_adjust_dpi=True` to prevent memory issues with large documents
