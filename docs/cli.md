# CLI Usage

The Kreuzberg CLI provides a convenient command-line interface for text extraction from documents.

## Installation

Install Kreuzberg with CLI support:

```bash
pip install kreuzberg[cli]
```

Or install all optional dependencies:

```bash
pip install kreuzberg[all]
```

## Basic Usage

### Extract from a file

```bash
kreuzberg extract document.pdf
```

### Extract to a file

```bash
kreuzberg extract document.pdf -o output.txt
```

### Extract from stdin

```bash
cat document.pdf | kreuzberg extract
# or
kreuzberg extract -
```

## Command-Line Options

### General Options

- `-o, --output PATH`: Output file path (default: stdout)
- `--output-format [text|json|markdown|tsv|hocr]`: Output format for extraction
- `--show-metadata`: Include metadata in output
- `-v, --verbose`: Verbose output for debugging

### Processing Options

- `--force-ocr`: Force OCR processing
- `--chunk-content`: Enable content chunking
- `--extract-tables`: Enable table extraction
- `--enable-table-detection`: Enable table extraction from scanned documents (with TSV format)
- `--max-chars INTEGER`: Maximum characters per chunk (default: 2000)
- `--max-overlap INTEGER`: Maximum overlap between chunks (default: 100)

### OCR Backend Options

- `--ocr-backend [tesseract|easyocr|paddleocr|none]`: OCR backend to use

#### Tesseract Options

- `--tesseract-lang TEXT`: Language(s) (e.g., 'eng+deu')
- `--tesseract-psm INTEGER`: PSM mode (0-13)
- `--tesseract-output-format [text|markdown|tsv|hocr]`: OCR output format (default: markdown)

#### EasyOCR Options

- `--easyocr-languages TEXT`: Language codes (comma-separated, e.g., 'en,de')

#### PaddleOCR Options

- `--paddleocr-languages TEXT`: Language codes (comma-separated, e.g., 'en,german')

## Configuration File

Kreuzberg can load configuration from a `pyproject.toml` or `kreuzberg.toml` file:

```toml
[tool.kreuzberg]
force_ocr = false

[tool.kreuzberg.ocr]
backend = "tesseract"
language = "eng+deu"
psm = 3
output_format = "markdown"

[tool.kreuzberg.chunking]
max_chars = 5000
max_overlap = 200

[tool.kreuzberg.tables]
detection_threshold = 0.7
structure_threshold = 0.5

[tool.kreuzberg.keywords]
top_k = 10

[tool.kreuzberg.entities]
# Entity extraction configuration
```

Or use a standalone `kreuzberg.toml`:

```toml
force_ocr = false

[ocr]
backend = "tesseract"
language = "eng+deu"
psm = 3

[chunking]
max_chars = 2000
max_overlap = 100

[tables]
detection_threshold = 0.7

[keywords]
top_k = 10
```

Use a specific config file:

```bash
kreuzberg extract document.pdf --config custom-config.toml
```

## OCR Output Format Examples

### Extract tables from scanned documents

```bash
# Extract tables using TSV format with table detection
kreuzberg extract invoice.png --tesseract-output-format tsv --enable-table-detection
```

### Fast text extraction

```bash
# Use text format for fastest extraction
kreuzberg extract document.pdf --tesseract-output-format text
```

### Get structured markdown output

```bash
# Markdown is the default format
kreuzberg extract document.jpg

# Or explicitly specify markdown
kreuzberg extract document.jpg --tesseract-output-format markdown
```

### Extract with position information

```bash
# Use hOCR format to get word positions
kreuzberg extract document.pdf --tesseract-output-format hocr -o output.html
```

## Examples

### Basic text extraction

```bash
kreuzberg extract report.pdf -o report.txt
```

### OCR with specific language

```bash
kreuzberg extract scan.jpg --force-ocr --tesseract-lang deu
```

### Extract tables to JSON

```bash
kreuzberg extract spreadsheet.pdf --extract-tables --output-format json -o tables.json
```

### Extract with metadata

```bash
kreuzberg extract document.pdf --show-metadata --output-format json
```

### Using EasyOCR backend

```bash
kreuzberg extract image.png --ocr-backend easyocr --easyocr-languages en,de
```

### Extract with chunking

```bash
kreuzberg extract large-document.pdf --chunk-content --max-chars 1000
```

## Module Execution

You can also run Kreuzberg as a Python module:

```bash
python -m kreuzberg extract document.pdf
```

## Command Reference

### `kreuzberg extract`

Extract text from a document.

**Usage:**

```bash
kreuzberg extract [OPTIONS] [FILE]
```

**Arguments:**

- `FILE`: Path to document or '-' for stdin (optional, defaults to stdin)

### `kreuzberg config`

Show current configuration.

**Usage:**

```bash
kreuzberg config [OPTIONS]
```

**Options:**

- `--config PATH`: Configuration file path

### `kreuzberg --version`

Show version information.

## Error Handling

The CLI provides clear error messages:

- Exit code 0: Success
- Exit code 1: General error (parsing, validation, etc.)
- Exit code 2: Missing dependency error

Use `--verbose` for detailed error information and stack traces.
