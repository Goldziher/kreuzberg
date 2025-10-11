# OCR Configuration

Kreuzberg offers comprehensive OCR configuration to extract text from images and scanned documents using multiple OCR engines.

## Quick Start

```python
from kreuzberg import extract_file, ExtractionConfig, TesseractConfig

# Extract text from an image with German language
result = await extract_file(
    "german_document.pdf",
    config=ExtractionConfig(
        ocr=TesseractConfig(language="deu"),
    ),
)
```

## OCR Engine Selection

Kreuzberg supports three OCR engines, configured using tagged unions (the config type determines the backend):

### Tesseract (Default)

```python
from kreuzberg import extract_file, ExtractionConfig, TesseractConfig, PSMMode

result = await extract_file(
    "document.pdf",
    config=ExtractionConfig(
        ocr=TesseractConfig(
            language="eng",
            psm=PSMMode.SINGLE_BLOCK,
            output_format="markdown",
        ),
    ),
)
```

Installation:

- **Ubuntu/Debian**: `sudo apt-get install tesseract-ocr`
- **macOS**: `brew install tesseract`
- **Windows**: Download from [GitHub releases](https://github.com/UB-Mannheim/tesseract/wiki)

### EasyOCR

```python
from kreuzberg import extract_file, ExtractionConfig, EasyOCRConfig

result = await extract_file(
    "document.jpg",
    config=ExtractionConfig(
        ocr=EasyOCRConfig(
            language=("en", "de"),  # Must be tuple
            device="cpu",
            confidence_threshold=0.5,
        ),
    ),
)
```

Installation: `pip install "kreuzberg[easyocr]"`

### PaddleOCR

```python
from kreuzberg import extract_file, ExtractionConfig, PaddleOCRConfig

result = await extract_file(
    "chinese_document.jpg",
    config=ExtractionConfig(
        ocr=PaddleOCRConfig(
            language="ch",  # Different language codes than Tesseract
            device="cpu",
        ),
    ),
)
```

Installation: `pip install "kreuzberg[paddleocr]"`

**Note**: PaddleOCR uses different language codes: `ch` (Chinese), `en` (English), `french`, `german`, `japan`, `korean`.

## Language Configuration

### Tesseract Languages

Specify language models for OCR:

```python
# Single language
config = ExtractionConfig(ocr=TesseractConfig(language="eng"))

# Multiple languages (faster first language is primary)
config = ExtractionConfig(ocr=TesseractConfig(language="eng+deu"))
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

#### Language Installation

For Tesseract to recognize languages other than English:

- **Ubuntu/Debian**: `sudo apt-get install tesseract-ocr-<lang-code>`
- **macOS**: `brew install tesseract-lang` (installs all languages)
- **Windows**: Download from [GitHub](https://github.com/tesseract-ocr/tessdata)

### EasyOCR Languages

```python
config = ExtractionConfig(
    ocr=EasyOCRConfig(
        language=("en", "de", "fr"),  # Tuple of language codes
    ),
)
```

### PaddleOCR Languages

```python
config = ExtractionConfig(
    ocr=PaddleOCRConfig(language="ch"),  # ch, en, french, german, japan, korean
)
```

## Page Segmentation Mode (Tesseract)

The `psm` parameter controls how Tesseract analyzes page layout:

```python
from kreuzberg import PSMMode

config = ExtractionConfig(
    ocr=TesseractConfig(psm=PSMMode.SINGLE_BLOCK),
)
```

### Available PSM Modes

| Mode          | Enum Value              | Description                                              | Best For                                       |
| ------------- | ----------------------- | -------------------------------------------------------- | ---------------------------------------------- |
| Auto Only     | `PSMMode.AUTO_ONLY`     | Automatic segmentation without orientation detection     | Modern documents (default - fastest)           |
| Automatic     | `PSMMode.AUTO`          | Automatic page segmentation with orientation detection   | Rotated/skewed documents                       |
| Single Block  | `PSMMode.SINGLE_BLOCK`  | Treat the image as a single text block                   | Simple layouts, preserving paragraph structure |
| Single Column | `PSMMode.SINGLE_COLUMN` | Assume a single column of text                           | Books, articles, single-column documents       |
| Single Line   | `PSMMode.SINGLE_LINE`   | Treat the image as a single text line                    | Receipts, labels, single-line text             |
| Single Word   | `PSMMode.SINGLE_WORD`   | Treat the image as a single word                         | Word recognition tasks                         |
| Sparse Text   | `PSMMode.SPARSE_TEXT`   | Find as much text as possible without assuming structure | Forms, tables, scattered text                  |

## Forcing OCR

By default, Kreuzberg only uses OCR for images and scanned PDFs. For searchable PDFs, it extracts text directly. Override this with `force_ocr`:

```python
# Force OCR even for searchable PDFs
result = await extract_file(
    "searchable.pdf",
    config=ExtractionConfig(force_ocr=True),
)
```

Use when:

- PDF contains both searchable text and images with text
- Embedded text has encoding/extraction issues
- You want consistent processing across all documents

## Output Formats (Tesseract)

Tesseract supports multiple output formats. **Markdown is the default** (since v3.5.0).

### Markdown (Default)

Structured output with preserved formatting:

```python
config = ExtractionConfig(ocr=TesseractConfig(output_format="markdown"))
```

### Text Format

Plain text extraction (fastest):

```python
config = ExtractionConfig(ocr=TesseractConfig(output_format="text"))
```

### hOCR Format

HTML-based output with word positions and bounding boxes:

```python
config = ExtractionConfig(ocr=TesseractConfig(output_format="hocr"))
```

### TSV Format

Tab-separated values with optional table detection:

```python
config = ExtractionConfig(
    ocr=TesseractConfig(
        output_format="tsv",
        enable_table_detection=True,
    ),
)
```

When using TSV with table detection enabled, tables are automatically extracted and available in `result.tables`.

## Performance Optimization

### Speed vs Quality Trade-offs

```python
# Default (optimized for modern documents)
config = ExtractionConfig(ocr=TesseractConfig())

# Maximum speed
config = ExtractionConfig(
    ocr=TesseractConfig(
        psm=PSMMode.AUTO_ONLY,
        output_format="text",
    ),
)

# Maximum accuracy (for degraded/historical documents)
config = ExtractionConfig(
    ocr=TesseractConfig(
        psm=PSMMode.AUTO,
        language="eng",
    ),
)
```

### DPI Configuration

Control image resolution for OCR processing:

```python
# Default with automatic DPI adjustment
config = ExtractionConfig(
    ocr=TesseractConfig(language="eng"),
    target_dpi=150,
    auto_adjust_dpi=True,
)

# High-quality processing
config = ExtractionConfig(
    ocr=TesseractConfig(language="eng"),
    target_dpi=300,
    max_image_dimension=50000,
)

# Speed-optimized
config = ExtractionConfig(
    ocr=TesseractConfig(language="eng"),
    target_dpi=120,
)
```

#### DPI Guidelines

- **72-120 DPI**: Fast processing, suitable for clean modern documents
- **150 DPI**: Default, good balance of speed and quality
- **200-300 DPI**: High quality, for small text or degraded documents
- **300+ DPI**: Maximum quality, very slow, for archival/fine print

### Device Selection (EasyOCR/PaddleOCR)

```python
# Use GPU if available
config = ExtractionConfig(
    ocr=EasyOCRConfig(
        language=("en",),
        device="cuda",  # or "mps" for Apple Silicon
    ),
)

# Force CPU
config = ExtractionConfig(
    ocr=EasyOCRConfig(
        language=("en",),
        device="cpu",
    ),
)
```

## Best Practices

1. **Language Selection**: Use the correct language model - wrong language significantly reduces accuracy
1. **PSM Selection**: Choose appropriate PSM for your document layout
1. **Engine Selection**:
    - **Tesseract**: General-purpose, best for printed text, many languages
    - **EasyOCR**: Better for scene text, handwriting, some non-Latin scripts
    - **PaddleOCR**: Excellent for Chinese and Asian languages
1. **DPI Settings**: Higher DPI = better quality but slower processing
1. **Output Format**: Use `text` for speed, `markdown` for structure, `tsv` for tables

## Advanced Configuration

### Confidence Thresholds (EasyOCR)

```python
config = ExtractionConfig(
    ocr=EasyOCRConfig(
        language=("en",),
        confidence_threshold=0.7,  # Filter low-confidence results
    ),
)
```

### Batch Size (EasyOCR)

```python
config = ExtractionConfig(
    ocr=EasyOCRConfig(
        language=("en",),
        batch_size=10,  # Process multiple images in parallel
    ),
)
```

### Table Detection (Tesseract TSV)

```python
config = ExtractionConfig(
    ocr=TesseractConfig(
        output_format="tsv",
        enable_table_detection=True,
    ),
)

result = await extract_file("scanned_receipt.jpg", config=config)

# Access extracted tables
for table in result.tables:
    print(f"Table: {table['text']}")
    # table['df'] contains Polars DataFrame
```

## Disabling OCR

Set `ocr=None` to disable OCR completely:

```python
config = ExtractionConfig(ocr=None)  # No OCR overhead for text documents
```

## Configuration Files

TOML configuration for OCR:

```toml
# kreuzberg.toml
force_ocr = false

[ocr]
backend = "tesseract"  # or "easyocr", "paddleocr"
language = "eng+deu"
psm = 6                # Page Segmentation Mode: 0-10 (6 = single uniform block)
output_format = "markdown"

# DPI settings
target_dpi = 150
auto_adjust_dpi = true
min_dpi = 72
max_dpi = 600
```

See the [Extraction Configuration](extraction-configuration.md) guide for complete TOML examples.
