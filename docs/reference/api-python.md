# Python API Reference

Complete reference for the Kreuzberg Python API.

## Installation

```bash title="Terminal"
pip install kreuzberg
```

**With EasyOCR:**

```bash title="Terminal"
pip install "kreuzberg[easyocr]"
```

**With PaddleOCR:**

```bash title="Terminal"
pip install "kreuzberg[paddleocr]"
```

**With API server:**

```bash title="Terminal"
pip install "kreuzberg[api]"
```

**With all features:**

```bash title="Terminal"
pip install "kreuzberg[all]"
```

## Core Functions

### extract_file_sync()

Extract content from a file (synchronous).

**Signature:**

```python title="Python"
def extract_file_sync(
    file_path: str | Path,
    mime_type: str | None = None,
    config: ExtractionConfig | None = None,
    *,
    easyocr_kwargs: dict[str, Any] | None = None,
    paddleocr_kwargs: dict[str, Any] | None = None,
) -> ExtractionResult
```

**Parameters:**

- `file_path` (str | Path): Path to the file to extract
- `mime_type` (str | None): Optional MIME type hint. If None, MIME type is auto-detected from file extension and content
- `config` (ExtractionConfig | None): Extraction configuration. Uses defaults if None
- `easyocr_kwargs` (dict | None): EasyOCR initialization options (languages, use_gpu, beam_width, etc.)
- `paddleocr_kwargs` (dict | None): PaddleOCR initialization options (lang, use_angle_cls, show_log, etc.)

**Returns:**

- `ExtractionResult`: Extraction result containing content, metadata, and tables

**Raises:**

- `KreuzbergError`: Base exception for all extraction errors
- `ValidationError`: Invalid configuration or file path
- `ParsingError`: Document parsing failure
- `OCRError`: OCR processing failure
- `MissingDependencyError`: Required system dependency not found

**Example - Basic usage:**

```python title="basic_extraction.py"
from kreuzberg import extract_file_sync

result = extract_file_sync("document.pdf")
print(result.content)
print(f"Pages: {result.metadata['page_count']}")
```

**Example - With OCR:**

```python title="with_ocr.py"
from kreuzberg import extract_file_sync, ExtractionConfig, OcrConfig

config = ExtractionConfig(
    ocr=OcrConfig(backend="tesseract", language="eng")
)
result = extract_file_sync("scanned.pdf", config=config)
```

**Example - With EasyOCR custom options:**

```python title="easyocr_custom.py"
from kreuzberg import extract_file_sync, ExtractionConfig, OcrConfig

config = ExtractionConfig(
    ocr=OcrConfig(backend="easyocr", language="eng")
)
result = extract_file_sync(
    "scanned.pdf",
    config=config,
    easyocr_kwargs={"use_gpu": True, "beam_width": 10}
)
```

---

### extract_file()

Extract content from a file (asynchronous).

**Signature:**

```python title="Python"
async def extract_file(
    file_path: str | Path,
    mime_type: str | None = None,
    config: ExtractionConfig | None = None,
    *,
    easyocr_kwargs: dict[str, Any] | None = None,
    paddleocr_kwargs: dict[str, Any] | None = None,
) -> ExtractionResult
```

**Parameters:**

Same as [`extract_file_sync()`](#extract_file_sync).

**Returns:**

- `ExtractionResult`: Extraction result containing content, metadata, and tables

**Examples:**

```python title="basic_extraction.py"
import asyncio
from kreuzberg import extract_file

async def main():
    result = await extract_file("document.pdf")
    print(result.content)

asyncio.run(main())
```

---

### extract_bytes_sync()

Extract content from bytes (synchronous).

**Signature:**

```python title="Python"
def extract_bytes_sync(
    data: bytes | bytearray,
    mime_type: str,
    config: ExtractionConfig | None = None,
    *,
    easyocr_kwargs: dict[str, Any] | None = None,
    paddleocr_kwargs: dict[str, Any] | None = None,
) -> ExtractionResult
```

**Parameters:**

- `data` (bytes | bytearray): File content as bytes or bytearray
- `mime_type` (str): MIME type of the data (required for format detection)
- `config` (ExtractionConfig | None): Extraction configuration. Uses defaults if None
- `easyocr_kwargs` (dict | None): EasyOCR initialization options
- `paddleocr_kwargs` (dict | None): PaddleOCR initialization options

**Returns:**

- `ExtractionResult`: Extraction result containing content, metadata, and tables

**Examples:**

```python title="basic_extraction.py"
from kreuzberg import extract_bytes_sync

with open("document.pdf", "rb") as f:
    data = f.read()

result = extract_bytes_sync(data, "application/pdf")
print(result.content)
```

---

### extract_bytes()

Extract content from bytes (asynchronous).

**Signature:**

```python title="Python"
async def extract_bytes(
    data: bytes | bytearray,
    mime_type: str,
    config: ExtractionConfig | None = None,
    *,
    easyocr_kwargs: dict[str, Any] | None = None,
    paddleocr_kwargs: dict[str, Any] | None = None,
) -> ExtractionResult
```

**Parameters:**

Same as [`extract_bytes_sync()`](#extract_bytes_sync).

**Returns:**

- `ExtractionResult`: Extraction result containing content, metadata, and tables

---

### batch_extract_files_sync()

Extract content from multiple files in parallel (synchronous).

**Signature:**

```python title="Python"
def batch_extract_files_sync(
    paths: list[str | Path],
    config: ExtractionConfig | None = None,
    *,
    easyocr_kwargs: dict[str, Any] | None = None,
    paddleocr_kwargs: dict[str, Any] | None = None,
) -> list[ExtractionResult]
```

**Parameters:**

- `paths` (list[str | Path]): List of file paths to extract
- `config` (ExtractionConfig | None): Extraction configuration applied to all files
- `easyocr_kwargs` (dict | None): EasyOCR initialization options
- `paddleocr_kwargs` (dict | None): PaddleOCR initialization options

**Returns:**

- `list[ExtractionResult]`: List of extraction results (one per file)

**Examples:**

```python title="basic_extraction.py"
from kreuzberg import batch_extract_files_sync

paths = ["doc1.pdf", "doc2.docx", "doc3.xlsx"]
results = batch_extract_files_sync(paths)

for path, result in zip(paths, results):
    print(f"{path}: {len(result.content)} characters")
```

---

### batch_extract_files()

Extract content from multiple files in parallel (asynchronous).

**Signature:**

```python title="Python"
async def batch_extract_files(
    paths: list[str | Path],
    config: ExtractionConfig | None = None,
    *,
    easyocr_kwargs: dict[str, Any] | None = None,
    paddleocr_kwargs: dict[str, Any] | None = None,
) -> list[ExtractionResult]
```

**Parameters:**

Same as [`batch_extract_files_sync()`](#batch_extract_files_sync).

**Returns:**

- `list[ExtractionResult]`: List of extraction results (one per file)

---

### batch_extract_bytes_sync()

Extract content from multiple byte arrays in parallel (synchronous).

**Signature:**

```python title="Python"
def batch_extract_bytes_sync(
    data_list: list[bytes | bytearray],
    mime_types: list[str],
    config: ExtractionConfig | None = None,
    *,
    easyocr_kwargs: dict[str, Any] | None = None,
    paddleocr_kwargs: dict[str, Any] | None = None,
) -> list[ExtractionResult]
```

**Parameters:**

- `data_list` (list[bytes | bytearray]): List of file contents as bytes/bytearray
- `mime_types` (list[str]): List of MIME types (one per data item, same length as data_list)
- `config` (ExtractionConfig | None): Extraction configuration applied to all items
- `easyocr_kwargs` (dict | None): EasyOCR initialization options
- `paddleocr_kwargs` (dict | None): PaddleOCR initialization options

**Returns:**

- `list[ExtractionResult]`: List of extraction results (one per data item)

---

### batch_extract_bytes()

Extract content from multiple byte arrays in parallel (asynchronous).

**Signature:**

```python title="Python"
async def batch_extract_bytes(
    data_list: list[bytes | bytearray],
    mime_types: list[str],
    config: ExtractionConfig | None = None,
    *,
    easyocr_kwargs: dict[str, Any] | None = None,
    paddleocr_kwargs: dict[str, Any] | None = None,
) -> list[ExtractionResult]
```

**Parameters:**

Same as [`batch_extract_bytes_sync()`](#batch_extract_bytes_sync).

**Returns:**

- `list[ExtractionResult]`: List of extraction results (one per data item)

---

## Configuration

### ExtractionConfig

Main configuration class for extraction operations.

**Fields:**

- `ocr` (OcrConfig | None): OCR configuration. Default: None (no OCR)
- `force_ocr` (bool): Force OCR even for text-based PDFs. Default: False
- `pdf_options` (PdfConfig | None): PDF-specific configuration. Default: None
- `chunking` (ChunkingConfig | None): Text chunking configuration. Default: None
- `language_detection` (LanguageDetectionConfig | None): Language detection configuration. Default: None
- `token_reduction` (TokenReductionConfig | None): Token reduction configuration. Default: None
- `image_extraction` (ImageExtractionConfig | None): Image extraction from documents. Default: None
- `post_processor` (PostProcessorConfig | None): Post-processing configuration. Default: None

**Example:**

```python title="config.py"
from kreuzberg import ExtractionConfig, OcrConfig, PdfConfig

config = ExtractionConfig(
    ocr=OcrConfig(backend="tesseract", language="eng"),
    force_ocr=False,
    pdf_options=PdfConfig(
        passwords=["password1", "password2"],
        extract_images=True
    )
)

result = extract_file_sync("document.pdf", config=config)
```

---

### OcrConfig

OCR processing configuration.

**Fields:**

- `backend` (str): OCR backend to use. Options: "tesseract", "easyocr", "paddleocr". Default: "tesseract"
- `language` (str): Language code for OCR (ISO 639-3). Default: "eng"
- `tesseract_config` (TesseractConfig | None): Tesseract-specific configuration. Default: None

**Example - Basic OCR:**

```python title="with_ocr.py"
from kreuzberg import OcrConfig

ocr_config = OcrConfig(backend="tesseract", language="eng")
```

**Example - With EasyOCR:**

```python title="with_ocr.py"
from kreuzberg import OcrConfig

ocr_config = OcrConfig(backend="easyocr", language="en")
```

---

### TesseractConfig

Tesseract OCR backend configuration.

**Fields:**

- `psm` (int): Page segmentation mode (0-13). Default: 3 (auto)
- `oem` (int): OCR engine mode (0-3). Default: 3 (LSTM only)
- `enable_table_detection` (bool): Enable table detection and extraction. Default: False
- `tessedit_char_whitelist` (str | None): Character whitelist (e.g., "0123456789" for digits only). Default: None
- `tessedit_char_blacklist` (str | None): Character blacklist. Default: None

**Example:**

```python title="basic_extraction.py"
from kreuzberg import OcrConfig, TesseractConfig

config = ExtractionConfig(
    ocr=OcrConfig(
        backend="tesseract",
        language="eng",
        tesseract_config=TesseractConfig(
            psm=6,
            enable_table_detection=True,
            tessedit_char_whitelist="0123456789"
        )
    )
)
```

---

### PdfConfig

PDF-specific configuration.

**Fields:**

- `passwords` (list[str] | None): List of passwords to try for encrypted PDFs. Default: None
- `extract_images` (bool): Extract images from PDF. Default: False
- `image_dpi` (int): DPI for image extraction. Default: 300

**Example:**

```python title="basic_extraction.py"
from kreuzberg import PdfConfig

pdf_config = PdfConfig(
    passwords=["password1", "password2"],
    extract_images=True,
    image_dpi=300
)
```

---

### ChunkingConfig

Text chunking configuration for splitting long documents.

**Fields:**

- `chunk_size` (int): Maximum chunk size in tokens. Default: 512
- `chunk_overlap` (int): Overlap between chunks in tokens. Default: 50
- `chunking_strategy` (str): Chunking strategy. Options: "fixed", "semantic". Default: "fixed"

**Example:**

```python title="basic_extraction.py"
from kreuzberg import ChunkingConfig

chunking_config = ChunkingConfig(
    chunk_size=1024,
    chunk_overlap=100,
    chunking_strategy="semantic"
)
```

---

### LanguageDetectionConfig

Language detection configuration.

**Fields:**

- `enabled` (bool): Enable language detection. Default: True
- `confidence_threshold` (float): Minimum confidence threshold (0.0-1.0). Default: 0.5

**Example:**

```python title="basic_extraction.py"
from kreuzberg import LanguageDetectionConfig

lang_config = LanguageDetectionConfig(
    enabled=True,
    confidence_threshold=0.7
)
```

---

### ImageExtractionConfig

Image extraction configuration.

**Fields:**

- `enabled` (bool): Enable image extraction from documents. Default: False
- `min_width` (int): Minimum image width in pixels. Default: 100
- `min_height` (int): Minimum image height in pixels. Default: 100

---

### TokenReductionConfig

Token reduction configuration for compressing extracted text.

**Fields:**

- `enabled` (bool): Enable token reduction. Default: False
- `strategy` (str): Reduction strategy. Options: "whitespace", "stemming". Default: "whitespace"

---

### PostProcessorConfig

Post-processing configuration.

**Fields:**

- `enabled` (bool): Enable post-processing. Default: True
- `processors` (list[str]): List of processor names to enable. Default: all registered processors

---

### ImagePreprocessingConfig

Image preprocessing configuration for OCR.

**Fields:**

- `target_dpi` (int): Target DPI for image preprocessing. Default: 300
- `auto_rotate` (bool): Auto-rotate images based on orientation. Default: True
- `denoise` (bool): Apply denoising filter. Default: False

---

## Results & Types

### ExtractionResult

Result object returned by all extraction functions.

**Type Definition:**

```python title="Python"
class ExtractionResult(TypedDict):
    content: str
    mime_type: str
    metadata: Metadata
    tables: list[Table]
    detected_languages: list[str] | None
```

**Fields:**

- `content` (str): Extracted text content
- `mime_type` (str): MIME type of the processed document
- `metadata` (Metadata): Document metadata (format-specific fields)
- `tables` (list[Table]): List of extracted tables
- `detected_languages` (list[str] | None): List of detected language codes (ISO 639-1) if language detection is enabled

**Example:**

```python title="basic_extraction.py"
result = extract_file_sync("document.pdf")

print(f"Content: {result.content}")
print(f"MIME type: {result.mime_type}")
print(f"Page count: {result.metadata.get('page_count')}")
print(f"Tables: {len(result.tables)}")

if result.detected_languages:
    print(f"Languages: {', '.join(result.detected_languages)}")
```

---

### Metadata

Strongly-typed metadata dictionary. Fields vary by document format.

**Common Fields:**

- `language` (str): Document language (ISO 639-1 code)
- `date` (str): Document date (ISO 8601 format)
- `subject` (str): Document subject
- `format_type` (str): Format discriminator ("pdf", "excel", "email", etc.)

**PDF-Specific Fields** (when `format_type == "pdf"`):

- `title` (str): PDF title
- `author` (str): PDF author
- `page_count` (int): Number of pages
- `creation_date` (str): Creation date (ISO 8601)
- `modification_date` (str): Modification date (ISO 8601)
- `creator` (str): Creator application
- `producer` (str): Producer application
- `keywords` (str): PDF keywords
- `subject` (str): PDF subject

**Excel-Specific Fields** (when `format_type == "excel"`):

- `sheet_count` (int): Number of sheets
- `sheet_names` (list[str]): List of sheet names

**Email-Specific Fields** (when `format_type == "email"`):

- `from_email` (str): Sender email address
- `from_name` (str): Sender name
- `to_emails` (list[str]): Recipient email addresses
- `cc_emails` (list[str]): CC email addresses
- `bcc_emails` (list[str]): BCC email addresses
- `message_id` (str): Email message ID
- `attachments` (list[str]): List of attachment filenames

**Example:**

```python title="basic_extraction.py"
result = extract_file_sync("document.pdf")
metadata = result.metadata

if metadata.get("format_type") == "pdf":
    print(f"Title: {metadata.get('title')}")
    print(f"Author: {metadata.get('author')}")
    print(f"Pages: {metadata.get('page_count')}")
```

See the Types Reference for complete metadata field documentation.

---

### Table

Extracted table structure.

**Type Definition:**

```python title="Python"
class Table(TypedDict):
    cells: list[list[str]]
    markdown: str
    page_number: int
```

**Fields:**

- `cells` (list[list[str]]): 2D array of table cells (rows x columns)
- `markdown` (str): Table rendered as markdown
- `page_number` (int): Page number where table was found

**Example:**

```python title="basic_extraction.py"
result = extract_file_sync("invoice.pdf")

for table in result.tables:
    print(f"Table on page {table.page_number}:")
    print(table.markdown)
    print()
```

---

### ExtractedTable

Deprecated alias for `Table`. Use `Table` instead.

---

## Extensibility

### Custom Post-Processors

Create custom post-processors to add processing logic to the extraction pipeline.

**Protocol:**

```python title="Python"
from kreuzberg import PostProcessorProtocol, ExtractionResult

class PostProcessorProtocol:
    def name(self) -> str:
        """Return unique processor name"""
        ...

    def process(self, result: ExtractionResult) -> ExtractionResult:
        """Process extraction result and return modified result"""
        ...

    def processing_stage(self) -> str:
        """Return processing stage: 'early', 'middle', or 'late'"""
        ...
```

**Example:**

```python title="basic_extraction.py"
from kreuzberg import (
    PostProcessorProtocol,
    ExtractionResult,
    register_post_processor
)

class CustomProcessor:
    def name(self) -> str:
        return "custom_processor"

    def process(self, result: ExtractionResult) -> ExtractionResult:
        # Add custom field to metadata
        result["metadata"]["custom_field"] = "custom_value"
        return result

    def processing_stage(self) -> str:
        return "middle"

# Register the processor
register_post_processor(CustomProcessor())

# Now all extractions will use this processor
result = extract_file_sync("document.pdf")
print(result.metadata["custom_field"])  # "custom_value"
```

**Managing Processors:**

```python title="basic_extraction.py"
from kreuzberg import (
    register_post_processor,
    unregister_post_processor,
    clear_post_processors
)

# Register
register_post_processor(CustomProcessor())

# Unregister by name
unregister_post_processor("custom_processor")

# Clear all processors
clear_post_processors()
```

---

### Custom Validators

Create custom validators to validate extraction results.

**Functions:**

```python title="custom_validator.py"
from kreuzberg import register_validator, unregister_validator, clear_validators

# Register a validator
register_validator(validator)

# Unregister by name
unregister_validator("validator_name")

# Clear all validators
clear_validators()
```

---

## Error Handling

All errors inherit from **`KreuzbergError`**. See [Error Handling Reference](errors.md) for complete documentation.

**Exception Hierarchy:**

- **`KreuzbergError`** — Base exception for all extraction errors
  - `ValidationError` — Invalid configuration or input
  - `ParsingError` — Document parsing failure
  - `OCRError` — OCR processing failure
  - `MissingDependencyError` — Missing optional dependency

**Example:**

```python title="error_handling.py"
from kreuzberg import (
    extract_file_sync,
    KreuzbergError,
    ValidationError,
    ParsingError,
    MissingDependencyError
)

try:
    result = extract_file_sync("document.pdf")
except ValidationError as e:
    print(f"Invalid input: {e}")
except ParsingError as e:
    print(f"Failed to parse document: {e}")
except MissingDependencyError as e:
    print(f"Missing dependency: {e}")
    print(f"Install with: {e.install_command}")
except KreuzbergError as e:
    print(f"Extraction failed: {e}")
```

See [Error Handling Reference](errors.md) for detailed error documentation and best practices.

---

## Version Information

```python title="basic_extraction.py"
import kreuzberg

print(kreuzberg.__version__)
```
