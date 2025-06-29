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

### Automatic Language Detection

Kreuzberg can automatically detect the language of your documents and configure OCR accordingly. This feature uses the `fast-langdetect` library for high-performance language detection.

#### Installation

To use automatic language detection, install the optional dependency:

```bash
pip install "kreuzberg[language-detection]"
```

#### Usage

Enable automatic language detection by setting `auto_detect_language=True` in your `ExtractionConfig`:

```python
from kreuzberg import extract_file, ExtractionConfig

# Enable automatic language detection
config = ExtractionConfig(auto_detect_language=True)
result = await extract_file("document.pdf", config=config)

# Access detected languages
print(f"Detected languages: {result.detected_languages}")
```

#### How It Works

1. **Text Extraction**: Kreuzberg first extracts text from your document
2. **Language Detection**: If `auto_detect_language=True`, it analyzes the extracted text to identify the language(s)
3. **OCR Configuration**: The detected language(s) are automatically configured for the OCR backend
4. **Result Storage**: Detected languages are stored in `result.detected_languages`

#### Supported Languages

The language detection supports over 50 languages including:

- **European**: English, German, French, Spanish, Italian, Portuguese, Dutch, Swedish, Norwegian, Danish, Finnish, Polish, Czech, Hungarian, Romanian, Bulgarian, Croatian, Serbian, Slovak, Slovenian
- **Asian**: Chinese (Simplified/Traditional), Japanese, Korean, Thai, Vietnamese, Indonesian, Malay, Filipino
- **Middle Eastern**: Arabic, Hebrew, Persian, Turkish
- **Indian**: Hindi, Bengali, Tamil, Telugu, Marathi, Gujarati, Kannada, Malayalam, Punjabi, Urdu
- **African**: Swahili, Afrikaans, Amharic, Zulu, Xhosa
- **And many more...**

#### Multi-Language Documents

For documents containing multiple languages, the detection returns the most probable languages in order of confidence:

```python
# Document with mixed English and German content
result = await extract_file("multilingual.pdf", config=ExtractionConfig(auto_detect_language=True))
print(result.detected_languages)  # Output: ['en', 'de']
```

#### OCR Backend Integration

The detected languages are automatically mapped to the appropriate parameters for each OCR backend:

- **Tesseract**: Uses `language` parameter (e.g., `"eng+deu"`)
- **EasyOCR**: Uses `language_list` parameter (e.g., `["en", "de"]`)
- **PaddleOCR**: Uses `language` parameter (e.g., `"en"` for the primary language)

#### Error Handling

If language detection is enabled but the `fast-langdetect` library is not installed, Kreuzberg will raise a `MissingDependencyError`:

```python
try:
    result = await extract_file("document.pdf", config=ExtractionConfig(auto_detect_language=True))
except MissingDependencyError as e:
    print(f"Install language detection: pip install 'kreuzberg[language-detection]'")
```

#### Performance Considerations

- **Caching**: Language detection results are cached to avoid redundant processing
- **Accuracy**: Detection accuracy improves with longer text samples
- **Fallback**: If detection fails, OCR backends fall back to their default language settings

#### Best Practices

- **Text Length**: Language detection works best with at least 50-100 characters of text
- **Mixed Content**: For documents with multiple languages, consider the primary language for OCR configuration
- **Performance**: Enable language detection only when needed, as it adds processing time
- **Fallback**: Always have a fallback language configuration for critical applications

### Page Segmentation Mode (PSM)

The `psm` parameter in a [`TesseractConfig`](../api-reference/ocr-configuration.md#tesseractconfig) object controls how Tesseract analyzes the layout of the page:

```python
from kreuzberg import extract_file, ExtractionConfig, TesseractConfig, PSMMode

# Extract text from a document with a simple layout
result = await extract_file("document.pdf", config=ExtractionConfig(ocr_config=TesseractConfig(psm=PSMMode.SINGLE_BLOCK)))
```

#### Available PSM Modes

| Mode                 | Enum Value                | Description                                              | Best For                                       |
| -------------------- | ------------------------- | -------------------------------------------------------- | ---------------------------------------------- |
| Automatic            | `PSMMode.AUTO`            | Automatic page segmentation with orientation detection   | General purpose (default)                      |
| Single Block         | `PSMMode.SINGLE_BLOCK`    | Treat the image as a single text block                   | Simple layouts, preserving paragraph structure |
| Single Line          | `PSMMode.SINGLE_LINE`     | Treat the image as a single text line                    | Receipts, labels, single-line text             |
| Single Word          | `PSMMode.SINGLE_WORD`     | Treat the image as a single word                         | Word recognition tasks                         |
| Single Character     | `PSMMode.SINGLE_CHAR`     | Treat the image as a single character                    | Character recognition tasks                    |
| Sparse Text          | `PSMMode.SPARSE_TEXT`     | Find as much text as possible without assuming structure | Forms, tables, scattered text                  |
| Sparse Text with OSD | `PSMMode.SPARSE_TEXT_OSD` | Like SPARSE_TEXT with orientation detection              | Complex layouts with varying text orientation  |

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
    config=ExtractionConfig(
        ocr_backend="easyocr", ocr_config=EasyOCRConfig(language_list=["en"])  # EasyOCR uses different language codes
    ),
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

## Performance Optimization

OCR performance and parallel processing can be controlled through process handlers and extraction hooks which are configured in the `ExtractionConfig` object. The default configuration handles performance optimization automatically.

This is useful for:

- Limiting resource usage on systems with limited memory
- Optimizing performance on systems with many CPU cores
- Balancing OCR tasks with other application workloads

## Best Practices

- **Language Selection**: Always specify the correct language for your documents to improve OCR accuracy
- **PSM Mode Selection**: Choose the appropriate PSM mode based on your document layout:
    - Use `PSM.SINGLE_BLOCK` for documents with simple layouts
    - Use `PSM.SPARSE_TEXT` for forms or documents with tables
    - Use `PSM.SINGLE_LINE` for receipts or labels
- **Image Quality**: For best results, ensure images are:
    - High resolution (at least 300 DPI)
    - Well-lit with good contrast
    - Not skewed or rotated
- **Performance**: For batch processing, adjust `max_processes` based on your system's capabilities
