# Configuration Reference

This page provides a comprehensive reference for all Kreuzberg configuration types. For usage guides and examples, see the [Configuration Guide](../guides/configuration.md).

## Overview

Kreuzberg supports multiple configuration methods:

1. **TOML files** - Preferred format, clear syntax
2. **YAML files** - Alternative format
3. **JSON files** - For programmatic generation
4. **Programmatic** - Direct object instantiation

### Configuration Discovery

Kreuzberg automatically discovers configuration files in this order:

1. Current directory: `./kreuzberg.{toml,yaml,yml,json}`
2. User config: `~/.config/kreuzberg/config.{toml,yaml,yml,json}`
3. System config: `/etc/kreuzberg/config.{toml,yaml,yml,json}`

For complete examples, see the [examples directory](https://github.com/Goldziher/kreuzberg/tree/main/examples).

---

## ExtractionConfig

Main extraction configuration controlling all aspects of document processing.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `use_cache` | `bool` | `true` | Enable caching of extraction results for faster re-processing |
| `enable_quality_processing` | `bool` | `true` | Enable quality post-processing (deduplication, mojibake fixing, etc.) |
| `force_ocr` | `bool` | `false` | Force OCR even for searchable PDFs with text layers |
| `ocr` | `OcrConfig?` | `None` | OCR configuration (if None, OCR disabled) |
| `pdf_options` | `PdfConfig?` | `None` | PDF-specific configuration options |
| `images` | `ImageExtractionConfig?` | `None` | Image extraction configuration |
| `chunking` | `ChunkingConfig?` | `None` | Text chunking configuration for splitting into chunks |
| `token_reduction` | `TokenReductionConfig?` | `None` | Token reduction configuration for optimizing LLM context |
| `language_detection` | `LanguageDetectionConfig?` | `None` | Automatic language detection configuration |
| `postprocessor` | `PostProcessorConfig?` | `None` | Post-processing pipeline configuration |
| `max_concurrent_extractions` | `int?` | `None` | Maximum concurrent batch extractions (defaults to num_cpus * 2) |

### Example

=== "C#"

    ```csharp
    using Kreuzberg;

    var config = new ExtractionConfig
    {
        UseCache = true,
        EnableQualityProcessing = true,
        ForceOcr = false,
    };

    var result = KreuzbergClient.ExtractFileSync("document.pdf", config);
    ```

=== "Go"

    --8<-- "snippets/go/config_basic.md"

=== "Java"

    --8<-- "snippets/java/config_basic.md"

=== "Python"

    --8<-- "snippets/python/config_basic.md"

=== "Ruby"

    --8<-- "snippets/ruby/config_basic.md"

=== "Rust"

    --8<-- "snippets/rust/config_basic.md"

=== "TypeScript"

    --8<-- "snippets/typescript/config_basic.md"

---

## OcrConfig

Configuration for OCR (Optical Character Recognition) processing on images and scanned PDFs.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `backend` | `str` | `"tesseract"` | OCR backend to use: `"tesseract"`, `"easyocr"`, `"paddleocr"` |
| `language` | `str` | `"eng"` | Language code(s) for OCR, e.g., `"eng"`, `"eng+fra"`, `"eng+deu+fra"` |
| `tesseract_config` | `TesseractConfig?` | `None` | Tesseract-specific configuration options |

### Example

=== "C#"

    --8<-- "snippets/csharp/config_ocr.md"

=== "Go"

    --8<-- "snippets/go/config_ocr.md"

=== "Java"

    --8<-- "snippets/java/config_ocr.md"

=== "Python"

    --8<-- "snippets/python/config_ocr.md"

=== "Ruby"

    --8<-- "snippets/ruby/config_ocr.md"

=== "Rust"

    --8<-- "snippets/rust/config_ocr.md"

=== "TypeScript"

    --8<-- "snippets/typescript/config_ocr.md"

---

## TesseractConfig

Tesseract OCR engine configuration with fine-grained control over recognition parameters.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `language` | `str` | `"eng"` | Language code(s), e.g., `"eng"`, `"eng+fra"` |
| `psm` | `int` | `3` | Page Segmentation Mode (0-13, see below) |
| `output_format` | `str` | `"markdown"` | Output format: `"text"`, `"markdown"`, `"hocr"` |
| `oem` | `int` | `3` | OCR Engine Mode (0-3, see below) |
| `min_confidence` | `float` | `0.0` | Minimum confidence threshold (0.0-100.0) |
| `preprocessing` | `ImagePreprocessingConfig?` | `None` | Image preprocessing configuration |
| `enable_table_detection` | `bool` | `true` | Enable automatic table detection and reconstruction |
| `table_min_confidence` | `float` | `0.0` | Minimum confidence for table cell recognition (0.0-1.0) |
| `table_column_threshold` | `int` | `50` | Pixel threshold for detecting table columns |
| `table_row_threshold_ratio` | `float` | `0.5` | Row threshold ratio for table detection (0.0-1.0) |
| `use_cache` | `bool` | `true` | Enable OCR result caching for faster re-processing |
| `classify_use_pre_adapted_templates` | `bool` | `true` | Use pre-adapted templates for character classification |
| `language_model_ngram_on` | `bool` | `false` | Enable N-gram language model for better word recognition |
| `tessedit_dont_blkrej_good_wds` | `bool` | `true` | Don't reject good words during block-level processing |
| `tessedit_dont_rowrej_good_wds` | `bool` | `true` | Don't reject good words during row-level processing |
| `tessedit_enable_dict_correction` | `bool` | `true` | Enable dictionary-based word correction |
| `tessedit_char_whitelist` | `str` | `""` | Allowed characters (empty = all allowed) |
| `tessedit_char_blacklist` | `str` | `""` | Forbidden characters (empty = none forbidden) |
| `tessedit_use_primary_params_model` | `bool` | `true` | Use primary language params model |
| `textord_space_size_is_variable` | `bool` | `true` | Enable variable-width space detection |
| `thresholding_method` | `bool` | `false` | Use adaptive thresholding method |

### Page Segmentation Modes (PSM)

- `0`: Orientation and script detection only (no OCR)
- `1`: Automatic page segmentation with OSD (Orientation and Script Detection)
- `2`: Automatic page segmentation (no OSD, no OCR)
- `3`: Fully automatic page segmentation (default, best for most documents)
- `4`: Single column of text of variable sizes
- `5`: Single uniform block of vertically aligned text
- `6`: Single uniform block of text (best for clean documents)
- `7`: Single text line
- `8`: Single word
- `9`: Single word in a circle
- `10`: Single character
- `11`: Sparse text with no particular order (best for forms, invoices)
- `12`: Sparse text with OSD
- `13`: Raw line (bypass Tesseract's layout analysis)

### OCR Engine Modes (OEM)

- `0`: Legacy Tesseract engine only (pre-2016)
- `1`: Neural nets LSTM engine only (recommended for best quality)
- `2`: Legacy + LSTM engines combined
- `3`: Default based on what's available (recommended for compatibility)

### Example

=== "C#"

    --8<-- "snippets/csharp/tesseract_config.md"

=== "Go"

    --8<-- "snippets/go/tesseract_config.md"

=== "Java"

    --8<-- "snippets/java/tesseract_config.md"

=== "Python"

    --8<-- "snippets/python/tesseract_config.md"

=== "Ruby"

    --8<-- "snippets/ruby/tesseract_config.md"

=== "Rust"

    --8<-- "snippets/rust/tesseract_config.md"

=== "TypeScript"

    --8<-- "snippets/typescript/tesseract_config.md"


---

## ChunkingConfig

Configuration for splitting extracted text into overlapping chunks, useful for vector databases and LLM processing.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `max_chars` | `int` | `1000` | Maximum characters per chunk |
| `max_overlap` | `int` | `200` | Overlap between consecutive chunks in characters |
| `embedding` | `EmbeddingConfig?` | `None` | Optional embedding generation for each chunk |
| `preset` | `str?` | `None` | Chunking preset: `"small"` (500/100), `"medium"` (1000/200), `"large"` (2000/400) |

### Example

=== "C#"

    --8<-- "snippets/csharp/chunking.md"

=== "Go"

    --8<-- "snippets/go/chunking.md"

=== "Java"

    --8<-- "snippets/java/chunking.md"

=== "Python"

    --8<-- "snippets/python/chunking.md"

=== "Ruby"

    --8<-- "snippets/ruby/chunking.md"

=== "Rust"

    --8<-- "snippets/rust/chunking.md"

=== "TypeScript"

    --8<-- "snippets/typescript/chunking.md"

---

## LanguageDetectionConfig

Configuration for automatic language detection in extracted text.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | `bool` | `true` | Enable language detection |
| `min_confidence` | `float` | `0.8` | Minimum confidence threshold (0.0-1.0) for reporting detected languages |
| `detect_multiple` | `bool` | `false` | Detect multiple languages (vs. dominant language only) |

### Example

=== "C#"

    --8<-- "snippets/csharp/language_detection.md"

=== "Go"

    --8<-- "snippets/go/language_detection.md"

=== "Java"

    --8<-- "snippets/java/language_detection.md"

=== "Python"

    --8<-- "snippets/python/language_detection.md"

=== "Ruby"

    --8<-- "snippets/ruby/language_detection.md"

=== "Rust"

    --8<-- "snippets/rust/language_detection.md"

=== "TypeScript"

    --8<-- "snippets/typescript/language_detection.md"


---

## PdfConfig

PDF-specific extraction configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `extract_images` | `bool` | `false` | Extract embedded images from PDF pages |
| `extract_metadata` | `bool` | `true` | Extract PDF metadata (title, author, creation date, etc.) |
| `passwords` | `list[str]?` | `None` | List of passwords to try for encrypted PDFs (tries in order) |

### Example

=== "C#"

    --8<-- "snippets/csharp/pdf_config.md"

=== "Go"

    --8<-- "snippets/go/pdf_config.md"

=== "Java"

    --8<-- "snippets/java/pdf_config.md"

=== "Python"

    --8<-- "snippets/python/pdf_config.md"

=== "Ruby"

    --8<-- "snippets/ruby/pdf_config.md"

=== "Rust"

    --8<-- "snippets/rust/pdf_config.md"

=== "TypeScript"

    --8<-- "snippets/typescript/pdf_config.md"


---

## ImageExtractionConfig

Configuration for extracting and processing images from documents.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `extract_images` | `bool` | `true` | Extract images from documents |
| `target_dpi` | `int` | `300` | Target DPI for extracted/normalized images |
| `max_image_dimension` | `int` | `4096` | Maximum image dimension (width or height) in pixels |
| `auto_adjust_dpi` | `bool` | `true` | Automatically adjust DPI based on image size and content |
| `min_dpi` | `int` | `72` | Minimum DPI when auto-adjusting |
| `max_dpi` | `int` | `600` | Maximum DPI when auto-adjusting |

### Example

=== "C#"

    --8<-- "snippets/csharp/image_extraction.md"

=== "Go"

    --8<-- "snippets/go/image_extraction.md"

=== "Java"

    --8<-- "snippets/java/image_extraction.md"

=== "Python"

    --8<-- "snippets/python/image_extraction.md"

=== "Ruby"

    --8<-- "snippets/ruby/image_extraction.md"

=== "Rust"

    --8<-- "snippets/rust/image_extraction.md"

=== "TypeScript"

    --8<-- "snippets/typescript/image_extraction.md"

---

## ImagePreprocessingConfig

Image preprocessing configuration for improving OCR quality on scanned documents.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `target_dpi` | `int` | `300` | Target DPI for OCR processing (300 standard, 600 for small text) |
| `auto_rotate` | `bool` | `true` | Auto-detect and correct image rotation |
| `deskew` | `bool` | `true` | Correct skew (tilted images) |
| `denoise` | `bool` | `false` | Apply noise reduction filter |
| `contrast_enhance` | `bool` | `false` | Enhance image contrast for better text visibility |
| `binarization_method` | `str` | `"otsu"` | Binarization method: `"otsu"`, `"sauvola"`, `"adaptive"`, `"none"` |
| `invert_colors` | `bool` | `false` | Invert colors (useful for white text on black background) |

### Example

=== "C#"

    --8<-- "snippets/csharp/image_preprocessing.md"

=== "Go"

    --8<-- "snippets/go/image_preprocessing.md"

=== "Java"

    --8<-- "snippets/java/image_preprocessing.md"

=== "Python"

    --8<-- "snippets/python/image_preprocessing.md"

=== "Ruby"

    --8<-- "snippets/ruby/image_preprocessing.md"

=== "Rust"

    --8<-- "snippets/rust/image_preprocessing.md"

=== "TypeScript"

    --8<-- "snippets/typescript/image_preprocessing.md"

---

## PostProcessorConfig

Configuration for the post-processing pipeline that runs after extraction.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | `bool` | `true` | Enable post-processing pipeline |
| `enabled_processors` | `list[str]?` | `None` | Specific processors to enable (if None, all enabled by default) |
| `disabled_processors` | `list[str]?` | `None` | Specific processors to disable (takes precedence over enabled_processors) |

Built-in post-processors include:

- `deduplication` - Remove duplicate text blocks
- `whitespace_normalization` - Normalize whitespace and line breaks
- `mojibake_fix` - Fix mojibake (encoding corruption)
- `quality_scoring` - Score and filter low-quality text

### Example

=== "C#"

    --8<-- "snippets/csharp/postprocessor_config.md"

=== "Go"

    --8<-- "snippets/go/postprocessor_config.md"

=== "Java"

    --8<-- "snippets/java/postprocessor_config.md"

=== "Python"

    --8<-- "snippets/python/postprocessor_config.md"

=== "Ruby"

    --8<-- "snippets/ruby/postprocessor_config.md"

=== "Rust"

    --8<-- "snippets/rust/postprocessor_config.md"

=== "TypeScript"

    --8<-- "snippets/typescript/postprocessor_config.md"

---

## TokenReductionConfig

Configuration for reducing token count in extracted text, useful for optimizing LLM context windows.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `mode` | `str` | `"off"` | Reduction mode: `"off"`, `"light"`, `"moderate"`, `"aggressive"`, `"maximum"` |
| `preserve_important_words` | `bool` | `true` | Preserve important words (capitalized, technical terms) during reduction |

### Reduction Modes

- `off`: No token reduction
- `light`: Remove redundant whitespace and line breaks (~5-10% reduction)
- `moderate`: Light + remove stopwords in low-information contexts (~15-25% reduction)
- `aggressive`: Moderate + abbreviate common phrases (~30-40% reduction)
- `maximum`: Aggressive + remove all stopwords (~50-60% reduction, may impact quality)

### Example

=== "C#"

    --8<-- "snippets/csharp/token_reduction.md"

=== "Go"

    --8<-- "snippets/go/token_reduction.md"

=== "Java"

    --8<-- "snippets/java/token_reduction.md"

=== "Python"

    --8<-- "snippets/python/token_reduction.md"

=== "Ruby"

    --8<-- "snippets/ruby/token_reduction.md"

=== "Rust"

    --8<-- "snippets/rust/token_reduction.md"

=== "TypeScript"

    --8<-- "snippets/typescript/token_reduction.md"

---

## Configuration File Examples

### TOML Format

```toml
# kreuzberg.toml
use_cache = true
enable_quality_processing = true
force_ocr = false

[ocr]
backend = "tesseract"
language = "eng+fra"

[ocr.tesseract_config]
psm = 6
oem = 1
min_confidence = 0.8
enable_table_detection = true

[ocr.tesseract_config.preprocessing]
target_dpi = 300
denoise = true
deskew = true
contrast_enhance = true
binarization_method = "otsu"

[pdf_options]
extract_images = true
extract_metadata = true
passwords = ["password1", "password2"]

[images]
extract_images = true
target_dpi = 200
max_image_dimension = 4096

[chunking]
max_chars = 1000
max_overlap = 200

[language_detection]
enabled = true
min_confidence = 0.8
detect_multiple = false

[token_reduction]
mode = "moderate"
preserve_important_words = true

[postprocessor]
enabled = true
```

### YAML Format

```yaml
# kreuzberg.yaml
use_cache: true
enable_quality_processing: true
force_ocr: false

ocr:
  backend: tesseract
  language: eng+fra
  tesseract_config:
    psm: 6
    oem: 1
    min_confidence: 0.8
    enable_table_detection: true
    preprocessing:
      target_dpi: 300
      denoise: true
      deskew: true
      contrast_enhance: true
      binarization_method: otsu

pdf_options:
  extract_images: true
  extract_metadata: true
  passwords:
    - password1
    - password2

images:
  extract_images: true
  target_dpi: 200
  max_image_dimension: 4096

chunking:
  max_chars: 1000
  max_overlap: 200

language_detection:
  enabled: true
  min_confidence: 0.8
  detect_multiple: false

token_reduction:
  mode: moderate
  preserve_important_words: true

postprocessor:
  enabled: true
```

### JSON Format

```json
{
  "use_cache": true,
  "enable_quality_processing": true,
  "force_ocr": false,
  "ocr": {
    "backend": "tesseract",
    "language": "eng+fra",
    "tesseract_config": {
      "psm": 6,
      "oem": 1,
      "min_confidence": 0.8,
      "enable_table_detection": true,
      "preprocessing": {
        "target_dpi": 300,
        "denoise": true,
        "deskew": true,
        "contrast_enhance": true,
        "binarization_method": "otsu"
      }
    }
  },
  "pdf_options": {
    "extract_images": true,
    "extract_metadata": true,
    "passwords": ["password1", "password2"]
  },
  "images": {
    "extract_images": true,
    "target_dpi": 200,
    "max_image_dimension": 4096
  },
  "chunking": {
    "max_chars": 1000,
    "max_overlap": 200
  },
  "language_detection": {
    "enabled": true,
    "min_confidence": 0.8,
    "detect_multiple": false
  },
  "token_reduction": {
    "mode": "moderate",
    "preserve_important_words": true
  },
  "postprocessor": {
    "enabled": true
  }
}
```

For complete working examples, see the [examples directory](https://github.com/Goldziher/kreuzberg/tree/main/examples).

---

## Best Practices

### When to Use Config Files vs Programmatic Config

**Use config files when:**

- Settings are shared across multiple scripts/applications
- Configuration needs to be version controlled
- Non-developers need to modify settings
- Deploying to multiple environments (dev/staging/prod)

**Use programmatic config when:**

- Settings vary per execution or are computed dynamically
- Configuration depends on runtime conditions
- Building SDKs or libraries that wrap Kreuzberg
- Rapid prototyping and experimentation

### Performance Considerations

**Caching:**

- Keep `use_cache=true` for repeated processing of the same files
- Cache is automatically invalidated when files change
- Cache location: `~/.cache/kreuzberg/` (configurable via environment)

**OCR Settings:**

- Lower `target_dpi` (e.g., 150-200) for faster processing of low-quality scans
- Higher `target_dpi` (e.g., 400-600) for small text or high-quality documents
- Disable `enable_table_detection` if tables aren't needed (10-20% speedup)
- Use `psm=6` for clean single-column documents (faster than `psm=3`)

**Batch Processing:**

- Set `max_concurrent_extractions` to balance speed and memory usage
- Default (num_cpus * 2) works well for most systems
- Reduce for memory-constrained environments
- Increase for I/O-bound workloads on systems with fast storage

**Token Reduction:**

- Use `"light"` or `"moderate"` modes for minimal quality impact
- `"aggressive"` and `"maximum"` modes may affect semantic meaning
- Benchmark with your specific LLM to measure quality vs. cost tradeoff

### Security Considerations

**API Keys and Secrets:**

- Never commit config files containing API keys or passwords to version control
- Use environment variables for sensitive data:
  ```bash
  export KREUZBERG_OCR_API_KEY="your-key-here"
  ```
- Add `kreuzberg.toml` to `.gitignore` if it contains secrets
- Use separate config files for development vs. production

**PDF Passwords:**

- `passwords` field attempts passwords in order until one succeeds
- Passwords are not logged or cached
- Use environment variables for sensitive passwords:
  ```python
  import os
  config = PdfConfig(passwords=[os.getenv("PDF_PASSWORD")])
  ```

**File System Access:**

- Kreuzberg only reads files you explicitly pass to extraction functions
- Cache directory permissions should be restricted to the running user
- Temporary files are automatically cleaned up after extraction

**Data Privacy:**

- Extraction results are never sent to external services (except explicit OCR backends)
- Tesseract OCR runs locally with no network access
- EasyOCR and PaddleOCR may download models on first run (cached locally)
- Consider disabling cache for sensitive documents requiring ephemeral processing

---

## Related Documentation

- [Configuration Guide](../guides/configuration.md) - Usage guide with examples
- [OCR Guide](../guides/ocr.md) - OCR-specific configuration and troubleshooting
- [Examples Directory](https://github.com/Goldziher/kreuzberg/tree/main/examples) - Complete working examples
