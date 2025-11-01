# Configuration

Kreuzberg's behavior is controlled through configuration objects. All settings are optional with sensible defaults, allowing you to configure only what you need.

## Configuration Methods

Kreuzberg supports four ways to configure extraction:

=== "Programmatic"

    ```python
    from kreuzberg import extract_file, ExtractionConfig, OcrConfig

    config = ExtractionConfig(
        use_cache=True,
        ocr=OcrConfig(backend="tesseract")
    )
    result = extract_file("document.pdf", config=config)
    ```

=== "TOML File"

    ```toml
    # kreuzberg.toml
    use_cache = true
    enable_quality_processing = true

    [ocr]
    backend = "tesseract"

    [ocr.tesseract]
    language = "eng"
    psm = 3
    ```

=== "YAML File"

    ```yaml
    # kreuzberg.yaml
    use_cache: true
    enable_quality_processing: true

    ocr:
      backend: tesseract
      tesseract:
        language: eng
        psm: 3
    ```

=== "JSON File"

    ```json
    {
      "use_cache": true,
      "enable_quality_processing": true,
      "ocr": {
        "backend": "tesseract",
        "tesseract": {
          "language": "eng",
          "psm": 3
        }
      }
    }
    ```

### Configuration Discovery

Kreuzberg automatically discovers configuration files in the following locations (in order):

1. Current directory: `./kreuzberg.{toml,yaml,yml,json}`
2. User config: `~/.config/kreuzberg/config.{toml,yaml,yml,json}`
3. System config: `/etc/kreuzberg/config.{toml,yaml,yml,json}`

=== "Python"

    ```python
    from kreuzberg import ExtractionConfig

    # Automatically discovers and loads config from standard locations
    config = ExtractionConfig.discover()
    result = extract_file("document.pdf", config=config)
    ```

=== "TypeScript"

    ```typescript
    import { ExtractionConfig, extractFile } from '@kreuzberg/sdk';

    // Automatically discovers and loads config from standard locations
    const config = await ExtractionConfig.discover();
    const result = await extractFile('document.pdf', { config });
    ```

=== "Rust"

    ```rust
    use kreuzberg::ExtractionConfig;

    // Automatically discovers and loads config from standard locations
    let config = ExtractionConfig::discover()?;
    let result = extract_file("document.pdf", None, &config).await?;
    ```

=== "Ruby"

    ```ruby
    require 'kreuzberg'

    # Automatically discovers and loads config from standard locations
    config = Kreuzberg::ExtractionConfig.discover
    result = Kreuzberg.extract_file('document.pdf', config: config)
    ```

## ExtractionConfig

The main configuration object controlling extraction behavior.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `use_cache` | `bool` | `true` | Enable caching of extraction results |
| `enable_quality_processing` | `bool` | `true` | Enable quality post-processing |
| `force_ocr` | `bool` | `false` | Force OCR even for text-based PDFs |
| `ocr` | `OcrConfig?` | `None` | OCR configuration (if None, OCR disabled) |
| `pdf` | `PdfConfig?` | `None` | PDF-specific configuration |
| `image_extraction` | `ImageExtractionConfig?` | `None` | Image extraction configuration |
| `chunking` | `ChunkingConfig?` | `None` | Text chunking configuration |
| `embedding` | `EmbeddingConfig?` | `None` | Embedding generation configuration |
| `token_reduction` | `TokenReductionConfig?` | `None` | Token reduction configuration |
| `language_detection` | `LanguageDetectionConfig?` | `None` | Language detection configuration |
| `post_processors` | `PostProcessorConfig?` | `None` | Post-processing pipeline configuration |
| `keywords` | `KeywordConfig?` | `None` | Keyword extraction configuration (requires feature flag) |

### Basic Example

=== "Python"

    ```python
    from kreuzberg import extract_file, ExtractionConfig

    config = ExtractionConfig(
        use_cache=True,
        enable_quality_processing=True
    )
    result = extract_file("document.pdf", config=config)
    ```

=== "TypeScript"

    ```typescript
    import { extractFile, ExtractionConfig } from '@kreuzberg/sdk';

    const config = new ExtractionConfig({
      useCache: true,
      enableQualityProcessing: true
    });
    const result = await extractFile('document.pdf', { config });
    ```

=== "Rust"

    ```rust
    use kreuzberg::{extract_file, ExtractionConfig};

    let config = ExtractionConfig {
        use_cache: true,
        enable_quality_processing: true,
        ..Default::default()
    };
    let result = extract_file("document.pdf", None, &config).await?;
    ```

=== "Ruby"

    ```ruby
    require 'kreuzberg'

    config = Kreuzberg::ExtractionConfig.new(
      use_cache: true,
      enable_quality_processing: true
    )
    result = Kreuzberg.extract_file('document.pdf', config: config)
    ```

## OcrConfig

Configuration for OCR processing. Set to enable OCR on images and scanned PDFs.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `backend` | `str` | `"tesseract"` | OCR backend: `"tesseract"`, `"easyocr"`, `"paddleocr"` |
| `tesseract` | `TesseractConfig?` | `None` | Tesseract-specific configuration |
| `image_preprocessing` | `ImagePreprocessingConfig?` | `None` | Image preprocessing configuration |

### Example

=== "Python"

    ```python
    from kreuzberg import ExtractionConfig, OcrConfig, TesseractConfig

    config = ExtractionConfig(
        ocr=OcrConfig(
            backend="tesseract",
            tesseract=TesseractConfig(language="eng+fra", psm=3)
        )
    )
    ```

=== "TypeScript"

    ```typescript
    import { ExtractionConfig, OcrConfig, TesseractConfig } from '@kreuzberg/sdk';

    const config = new ExtractionConfig({
      ocr: new OcrConfig({
        backend: 'tesseract',
        tesseract: new TesseractConfig({ language: 'eng+fra', psm: 3 })
      })
    });
    ```

=== "Rust"

    ```rust
    use kreuzberg::{ExtractionConfig, OcrConfig, TesseractConfig};

    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            tesseract: Some(TesseractConfig {
                language: "eng+fra".to_string(),
                psm: 3,
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    };
    ```

=== "Ruby"

    ```ruby
    require 'kreuzberg'

    config = Kreuzberg::ExtractionConfig.new(
      ocr: Kreuzberg::OcrConfig.new(
        backend: 'tesseract',
        tesseract: Kreuzberg::TesseractConfig.new(language: 'eng+fra', psm: 3)
      )
    )
    ```

## TesseractConfig

Tesseract OCR engine configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `language` | `str` | `"eng"` | Language code(s), e.g., `"eng"`, `"eng+fra"` |
| `psm` | `int` | `3` | Page segmentation mode (0-13) |
| `oem` | `int` | `3` | OCR engine mode (0-3) |
| `dpi` | `int?` | `None` | DPI for image processing |
| `min_confidence` | `float?` | `None` | Minimum confidence threshold (0.0-1.0) |
| `preserve_interword_spaces` | `bool` | `false` | Preserve multiple spaces between words |
| `char_whitelist` | `str?` | `None` | Allowed characters |
| `char_blacklist` | `str?` | `None` | Disallowed characters |
| `tessedit_char_whitelist` | `str?` | `None` | Tesseract variable: character whitelist |
| `tessedit_char_blacklist` | `str?` | `None` | Tesseract variable: character blacklist |
| `tessedit_pageseg_mode` | `int?` | `None` | Tesseract variable: page segmentation mode |
| `tessedit_ocr_engine_mode` | `int?` | `None` | Tesseract variable: OCR engine mode |
| `preserve_interword_spaces_flag` | `bool?` | `None` | Tesseract variable: preserve interword spaces |
| `textord_heavy_nr` | `bool?` | `None` | Tesseract variable: heavy noise removal |
| `textord_noise_rejwords` | `bool?` | `None` | Tesseract variable: reject noisy words |
| `textord_noise_rejrows` | `bool?` | `None` | Tesseract variable: reject noisy rows |
| `edges_use_new_outline_complexity` | `bool?` | `None` | Tesseract variable: use new outline complexity |
| `lstm_use_matrix` | `int?` | `None` | Tesseract variable: LSTM matrix usage |
| `user_words_file` | `str?` | `None` | Path to custom dictionary file |
| `user_patterns_file` | `str?` | `None` | Path to custom patterns file |

### Page Segmentation Modes (PSM)

- `0`: Orientation and script detection only
- `1`: Automatic page segmentation with OSD
- `2`: Automatic page segmentation (no OSD, no OCR)
- `3`: Fully automatic page segmentation (default)
- `4`: Single column of text
- `5`: Single uniform block of vertically aligned text
- `6`: Single uniform block of text
- `7`: Single text line
- `8`: Single word
- `9`: Single word in a circle
- `10`: Single character
- `11`: Sparse text, no particular order
- `12`: Sparse text with OSD
- `13`: Raw line (no assumptions about text layout)

### OCR Engine Modes (OEM)

- `0`: Legacy engine only
- `1`: Neural nets LSTM engine only
- `2`: Legacy + LSTM engines
- `3`: Default based on what's available (default)

### Example

=== "Python"

    ```python
    from kreuzberg import ExtractionConfig, OcrConfig, TesseractConfig

    config = ExtractionConfig(
        ocr=OcrConfig(
            tesseract=TesseractConfig(
                language="eng+fra+deu",
                psm=6,
                oem=1,
                dpi=300,
                min_confidence=0.8,
                char_whitelist="ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789 .,!?",
                preserve_interword_spaces=True
            )
        )
    )
    ```

=== "TypeScript"

    ```typescript
    import { ExtractionConfig, OcrConfig, TesseractConfig } from '@kreuzberg/sdk';

    const config = new ExtractionConfig({
      ocr: new OcrConfig({
        tesseract: new TesseractConfig({
          language: 'eng+fra+deu',
          psm: 6,
          oem: 1,
          dpi: 300,
          minConfidence: 0.8,
          charWhitelist: 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789 .,!?',
          preserveInterwordSpaces: true
        })
      })
    });
    ```

=== "Rust"

    ```rust
    use kreuzberg::{ExtractionConfig, OcrConfig, TesseractConfig};

    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            tesseract: Some(TesseractConfig {
                language: "eng+fra+deu".to_string(),
                psm: 6,
                oem: 1,
                dpi: Some(300),
                min_confidence: Some(0.8),
                char_whitelist: Some("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789 .,!?".to_string()),
                preserve_interword_spaces: true,
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    };
    ```

=== "Ruby"

    ```ruby
    require 'kreuzberg'

    config = Kreuzberg::ExtractionConfig.new(
      ocr: Kreuzberg::OcrConfig.new(
        tesseract: Kreuzberg::TesseractConfig.new(
          language: 'eng+fra+deu',
          psm: 6,
          oem: 1,
          dpi: 300,
          min_confidence: 0.8,
          char_whitelist: 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789 .,!?',
          preserve_interword_spaces: true
        )
      )
    )
    ```

## ImagePreprocessingConfig

Image preprocessing configuration for OCR.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `resize_factor` | `float` | `2.0` | Image resize factor for OCR |
| `denoise` | `bool` | `true` | Apply denoising |
| `deskew` | `bool` | `true` | Apply deskewing |
| `remove_background` | `bool` | `false` | Remove background |
| `enhance_contrast` | `bool` | `true` | Enhance contrast |
| `grayscale` | `bool` | `true` | Convert to grayscale |
| `binarize` | `bool` | `false` | Apply binarization |

### Example

=== "Python"

    ```python
    from kreuzberg import ExtractionConfig, OcrConfig, ImagePreprocessingConfig

    config = ExtractionConfig(
        ocr=OcrConfig(
            image_preprocessing=ImagePreprocessingConfig(
                resize_factor=3.0,
                denoise=True,
                deskew=True,
                enhance_contrast=True,
                binarize=True
            )
        )
    )
    ```

=== "TypeScript"

    ```typescript
    import { ExtractionConfig, OcrConfig, ImagePreprocessingConfig } from '@kreuzberg/sdk';

    const config = new ExtractionConfig({
      ocr: new OcrConfig({
        imagePreprocessing: new ImagePreprocessingConfig({
          resizeFactor: 3.0,
          denoise: true,
          deskew: true,
          enhanceContrast: true,
          binarize: true
        })
      })
    });
    ```

=== "Rust"

    ```rust
    use kreuzberg::{ExtractionConfig, OcrConfig, ImagePreprocessingConfig};

    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            image_preprocessing: Some(ImagePreprocessingConfig {
                resize_factor: 3.0,
                denoise: true,
                deskew: true,
                enhance_contrast: true,
                binarize: true,
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    };
    ```

=== "Ruby"

    ```ruby
    require 'kreuzberg'

    config = Kreuzberg::ExtractionConfig.new(
      ocr: Kreuzberg::OcrConfig.new(
        image_preprocessing: Kreuzberg::ImagePreprocessingConfig.new(
          resize_factor: 3.0,
          denoise: true,
          deskew: true,
          enhance_contrast: true,
          binarize: true
        )
      )
    )
    ```

## PdfConfig

PDF-specific extraction configuration.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `extract_images` | `bool` | `true` | Extract embedded images |
| `extract_tables` | `bool` | `true` | Extract tables |
| `password` | `str | list[str]?` | `None` | Password(s) for encrypted PDFs |

### Example

=== "Python"

    ```python
    from kreuzberg import ExtractionConfig, PdfConfig

    config = ExtractionConfig(
        pdf=PdfConfig(
            extract_images=True,
            extract_tables=True,
            password="secret123"
        )
    )
    ```

=== "TypeScript"

    ```typescript
    import { ExtractionConfig, PdfConfig } from '@kreuzberg/sdk';

    const config = new ExtractionConfig({
      pdf: new PdfConfig({
        extractImages: true,
        extractTables: true,
        password: 'secret123'
      })
    });
    ```

=== "Rust"

    ```rust
    use kreuzberg::{ExtractionConfig, PdfConfig};

    let config = ExtractionConfig {
        pdf: Some(PdfConfig {
            extract_images: true,
            extract_tables: true,
            password: Some("secret123".to_string()),
        }),
        ..Default::default()
    };
    ```

=== "Ruby"

    ```ruby
    require 'kreuzberg'

    config = Kreuzberg::ExtractionConfig.new(
      pdf: Kreuzberg::PdfConfig.new(
        extract_images: true,
        extract_tables: true,
        password: 'secret123'
      )
    )
    ```

## ImageExtractionConfig

Configuration for extracting images from documents.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `min_width` | `int` | `100` | Minimum image width in pixels |
| `min_height` | `int` | `100` | Minimum image height in pixels |
| `format` | `str` | `"png"` | Output format: `"png"`, `"jpeg"`, `"webp"` |
| `quality` | `int` | `90` | JPEG/WebP quality (1-100) |
| `extract_to_dir` | `str?` | `None` | Directory to save extracted images |
| `include_metadata` | `bool` | `true` | Include image metadata in results |

### Example

=== "Python"

    ```python
    from kreuzberg import ExtractionConfig, ImageExtractionConfig

    config = ExtractionConfig(
        image_extraction=ImageExtractionConfig(
            min_width=200,
            min_height=200,
            format="jpeg",
            quality=85,
            extract_to_dir="./extracted_images"
        )
    )
    ```

=== "TypeScript"

    ```typescript
    import { ExtractionConfig, ImageExtractionConfig } from '@kreuzberg/sdk';

    const config = new ExtractionConfig({
      imageExtraction: new ImageExtractionConfig({
        minWidth: 200,
        minHeight: 200,
        format: 'jpeg',
        quality: 85,
        extractToDir: './extracted_images'
      })
    });
    ```

=== "Rust"

    ```rust
    use kreuzberg::{ExtractionConfig, ImageExtractionConfig};

    let config = ExtractionConfig {
        image_extraction: Some(ImageExtractionConfig {
            min_width: 200,
            min_height: 200,
            format: "jpeg".to_string(),
            quality: 85,
            extract_to_dir: Some("./extracted_images".to_string()),
            ..Default::default()
        }),
        ..Default::default()
    };
    ```

=== "Ruby"

    ```ruby
    require 'kreuzberg'

    config = Kreuzberg::ExtractionConfig.new(
      image_extraction: Kreuzberg::ImageExtractionConfig.new(
        min_width: 200,
        min_height: 200,
        format: 'jpeg',
        quality: 85,
        extract_to_dir: './extracted_images'
      )
    )
    ```

## ChunkingConfig

Text chunking configuration for splitting extracted text into chunks.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `chunk_size` | `int` | `1000` | Maximum chunk size in characters |
| `chunk_overlap` | `int` | `200` | Overlap between chunks in characters |
| `separator` | `str` | `"\n\n"` | Separator for splitting text |
| `keep_separator` | `bool` | `true` | Keep separator in chunks |

### Example

=== "Python"

    ```python
    from kreuzberg import ExtractionConfig, ChunkingConfig

    config = ExtractionConfig(
        chunking=ChunkingConfig(
            chunk_size=500,
            chunk_overlap=50,
            separator="\n",
            keep_separator=False
        )
    )
    ```

=== "TypeScript"

    ```typescript
    import { ExtractionConfig, ChunkingConfig } from '@kreuzberg/sdk';

    const config = new ExtractionConfig({
      chunking: new ChunkingConfig({
        chunkSize: 500,
        chunkOverlap: 50,
        separator: '\n',
        keepSeparator: false
      })
    });
    ```

=== "Rust"

    ```rust
    use kreuzberg::{ExtractionConfig, ChunkingConfig};

    let config = ExtractionConfig {
        chunking: Some(ChunkingConfig {
            chunk_size: 500,
            chunk_overlap: 50,
            separator: "\n".to_string(),
            keep_separator: false,
        }),
        ..Default::default()
    };
    ```

=== "Ruby"

    ```ruby
    require 'kreuzberg'

    config = Kreuzberg::ExtractionConfig.new(
      chunking: Kreuzberg::ChunkingConfig.new(
        chunk_size: 500,
        chunk_overlap: 50,
        separator: "\n",
        keep_separator: false
      )
    )
    ```

## EmbeddingConfig

Configuration for generating embeddings from extracted text.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `str` | `"all-MiniLM-L6-v2"` | Embedding model name |
| `batch_size` | `int` | `32` | Batch size for embedding generation |
| `normalize` | `bool` | `true` | Normalize embeddings to unit length |

### Available Models

- `"all-MiniLM-L6-v2"`: Fast, 384-dimensional embeddings (default)
- `"all-mpnet-base-v2"`: High quality, 768-dimensional embeddings
- `"paraphrase-multilingual-MiniLM-L12-v2"`: Multilingual support

### Example

=== "Python"

    ```python
    from kreuzberg import ExtractionConfig, EmbeddingConfig

    config = ExtractionConfig(
        embedding=EmbeddingConfig(
            model="all-mpnet-base-v2",
            batch_size=16,
            normalize=True
        )
    )
    ```

=== "TypeScript"

    ```typescript
    import { ExtractionConfig, EmbeddingConfig } from '@kreuzberg/sdk';

    const config = new ExtractionConfig({
      embedding: new EmbeddingConfig({
        model: 'all-mpnet-base-v2',
        batchSize: 16,
        normalize: true
      })
    });
    ```

=== "Rust"

    ```rust
    use kreuzberg::{ExtractionConfig, EmbeddingConfig};

    let config = ExtractionConfig {
        embedding: Some(EmbeddingConfig {
            model: "all-mpnet-base-v2".to_string(),
            batch_size: 16,
            normalize: true,
        }),
        ..Default::default()
    };
    ```

=== "Ruby"

    ```ruby
    require 'kreuzberg'

    config = Kreuzberg::ExtractionConfig.new(
      embedding: Kreuzberg::EmbeddingConfig.new(
        model: 'all-mpnet-base-v2',
        batch_size: 16,
        normalize: true
      )
    )
    ```

## TokenReductionConfig

Configuration for reducing token count in extracted text.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `max_tokens` | `int` | `4096` | Maximum token count |
| `strategy` | `str` | `"truncate"` | Reduction strategy: `"truncate"`, `"summarize"` |

### Example

=== "Python"

    ```python
    from kreuzberg import ExtractionConfig, TokenReductionConfig

    config = ExtractionConfig(
        token_reduction=TokenReductionConfig(
            max_tokens=2048,
            strategy="truncate"
        )
    )
    ```

=== "TypeScript"

    ```typescript
    import { ExtractionConfig, TokenReductionConfig } from '@kreuzberg/sdk';

    const config = new ExtractionConfig({
      tokenReduction: new TokenReductionConfig({
        maxTokens: 2048,
        strategy: 'truncate'
      })
    });
    ```

=== "Rust"

    ```rust
    use kreuzberg::{ExtractionConfig, TokenReductionConfig};

    let config = ExtractionConfig {
        token_reduction: Some(TokenReductionConfig {
            max_tokens: 2048,
            strategy: "truncate".to_string(),
        }),
        ..Default::default()
    };
    ```

=== "Ruby"

    ```ruby
    require 'kreuzberg'

    config = Kreuzberg::ExtractionConfig.new(
      token_reduction: Kreuzberg::TokenReductionConfig.new(
        max_tokens: 2048,
        strategy: 'truncate'
      )
    )
    ```

## LanguageDetectionConfig

Configuration for automatic language detection.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `min_confidence` | `float` | `0.8` | Minimum confidence threshold (0.0-1.0) |
| `detect_all` | `bool` | `false` | Detect all languages (vs. dominant only) |
| `max_languages` | `int` | `3` | Maximum languages to detect |

### Example

=== "Python"

    ```python
    from kreuzberg import ExtractionConfig, LanguageDetectionConfig

    config = ExtractionConfig(
        language_detection=LanguageDetectionConfig(
            min_confidence=0.9,
            detect_all=True,
            max_languages=5
        )
    )
    ```

=== "TypeScript"

    ```typescript
    import { ExtractionConfig, LanguageDetectionConfig } from '@kreuzberg/sdk';

    const config = new ExtractionConfig({
      languageDetection: new LanguageDetectionConfig({
        minConfidence: 0.9,
        detectAll: true,
        maxLanguages: 5
      })
    });
    ```

=== "Rust"

    ```rust
    use kreuzberg::{ExtractionConfig, LanguageDetectionConfig};

    let config = ExtractionConfig {
        language_detection: Some(LanguageDetectionConfig {
            min_confidence: 0.9,
            detect_all: true,
            max_languages: 5,
        }),
        ..Default::default()
    };
    ```

=== "Ruby"

    ```ruby
    require 'kreuzberg'

    config = Kreuzberg::ExtractionConfig.new(
      language_detection: Kreuzberg::LanguageDetectionConfig.new(
        min_confidence: 0.9,
        detect_all: true,
        max_languages: 5
      )
    )
    ```

## PostProcessorConfig

Configuration for post-processing pipeline.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `remove_duplicates` | `bool` | `true` | Remove duplicate text |
| `normalize_whitespace` | `bool` | `true` | Normalize whitespace |
| `fix_encoding` | `bool` | `true` | Fix encoding issues (mojibake) |

### Example

=== "Python"

    ```python
    from kreuzberg import ExtractionConfig, PostProcessorConfig

    config = ExtractionConfig(
        post_processors=PostProcessorConfig(
            remove_duplicates=True,
            normalize_whitespace=True,
            fix_encoding=True
        )
    )
    ```

=== "TypeScript"

    ```typescript
    import { ExtractionConfig, PostProcessorConfig } from '@kreuzberg/sdk';

    const config = new ExtractionConfig({
      postProcessors: new PostProcessorConfig({
        removeDuplicates: true,
        normalizeWhitespace: true,
        fixEncoding: true
      })
    });
    ```

=== "Rust"

    ```rust
    use kreuzberg::{ExtractionConfig, PostProcessorConfig};

    let config = ExtractionConfig {
        post_processors: Some(PostProcessorConfig {
            remove_duplicates: true,
            normalize_whitespace: true,
            fix_encoding: true,
        }),
        ..Default::default()
    };
    ```

=== "Ruby"

    ```ruby
    require 'kreuzberg'

    config = Kreuzberg::ExtractionConfig.new(
      post_processors: Kreuzberg::PostProcessorConfig.new(
        remove_duplicates: true,
        normalize_whitespace: true,
        fix_encoding: true
      )
    )
    ```

## KeywordConfig

Configuration for keyword extraction (requires `keywords` feature flag).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `algorithm` | `str` | `"rake"` | Algorithm: `"rake"`, `"yake"`, `"textrank"` |
| `max_keywords` | `int` | `10` | Maximum keywords to extract |
| `min_score` | `float` | `0.5` | Minimum score threshold |
| `ngram_range` | `tuple[int, int]` | `(1, 3)` | N-gram range for keywords |
| `stopwords` | `list[str]?` | `None` | Custom stopwords list |
| `language` | `str` | `"en"` | Language for stopwords |
| `normalize` | `bool` | `true` | Normalize keywords (lowercase, etc.) |

!!! note "Feature Flag Required"
    Keyword extraction requires building Kreuzberg with the `keywords` feature flag enabled.

### Example

=== "Python"

    ```python
    from kreuzberg import ExtractionConfig, KeywordConfig

    config = ExtractionConfig(
        keywords=KeywordConfig(
            algorithm="rake",
            max_keywords=20,
            min_score=0.6,
            ngram_range=(1, 2),
            language="en",
            normalize=True
        )
    )
    ```

=== "TypeScript"

    ```typescript
    import { ExtractionConfig, KeywordConfig } from '@kreuzberg/sdk';

    const config = new ExtractionConfig({
      keywords: new KeywordConfig({
        algorithm: 'rake',
        maxKeywords: 20,
        minScore: 0.6,
        ngramRange: [1, 2],
        language: 'en',
        normalize: true
      })
    });
    ```

=== "Rust"

    ```rust
    use kreuzberg::{ExtractionConfig, KeywordConfig};

    let config = ExtractionConfig {
        keywords: Some(KeywordConfig {
            algorithm: "rake".to_string(),
            max_keywords: 20,
            min_score: 0.6,
            ngram_range: (1, 2),
            language: "en".to_string(),
            normalize: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    ```

=== "Ruby"

    ```ruby
    require 'kreuzberg'

    config = Kreuzberg::ExtractionConfig.new(
      keywords: Kreuzberg::KeywordConfig.new(
        algorithm: 'rake',
        max_keywords: 20,
        min_score: 0.6,
        ngram_range: [1, 2],
        language: 'en',
        normalize: true
      )
    )
    ```

## Complete Example

Here's a complete example showing all configuration options together:

=== "Python"

    ```python
    from kreuzberg import (
        extract_file,
        ExtractionConfig,
        OcrConfig,
        TesseractConfig,
        ImagePreprocessingConfig,
        PdfConfig,
        ImageExtractionConfig,
        ChunkingConfig,
        EmbeddingConfig,
        TokenReductionConfig,
        LanguageDetectionConfig,
        PostProcessorConfig,
    )

    config = ExtractionConfig(
        use_cache=True,
        enable_quality_processing=True,
        force_ocr=False,
        ocr=OcrConfig(
            backend="tesseract",
            tesseract=TesseractConfig(
                language="eng+fra",
                psm=3,
                oem=3,
                dpi=300,
                min_confidence=0.8,
            ),
            image_preprocessing=ImagePreprocessingConfig(
                resize_factor=2.0,
                denoise=True,
                deskew=True,
                enhance_contrast=True,
            ),
        ),
        pdf=PdfConfig(
            extract_images=True,
            extract_tables=True,
        ),
        image_extraction=ImageExtractionConfig(
            min_width=100,
            min_height=100,
            format="png",
            quality=90,
        ),
        chunking=ChunkingConfig(
            chunk_size=1000,
            chunk_overlap=200,
        ),
        embedding=EmbeddingConfig(
            model="all-MiniLM-L6-v2",
            batch_size=32,
        ),
        token_reduction=TokenReductionConfig(
            max_tokens=4096,
            strategy="truncate",
        ),
        language_detection=LanguageDetectionConfig(
            min_confidence=0.8,
            detect_all=False,
        ),
        post_processors=PostProcessorConfig(
            remove_duplicates=True,
            normalize_whitespace=True,
            fix_encoding=True,
        ),
    )

    result = extract_file("document.pdf", config=config)
    ```

=== "TOML"

    ```toml
    # kreuzberg.toml
    use_cache = true
    enable_quality_processing = true
    force_ocr = false

    [ocr]
    backend = "tesseract"

    [ocr.tesseract]
    language = "eng+fra"
    psm = 3
    oem = 3
    dpi = 300
    min_confidence = 0.8

    [ocr.image_preprocessing]
    resize_factor = 2.0
    denoise = true
    deskew = true
    enhance_contrast = true

    [pdf]
    extract_images = true
    extract_tables = true

    [image_extraction]
    min_width = 100
    min_height = 100
    format = "png"
    quality = 90

    [chunking]
    chunk_size = 1000
    chunk_overlap = 200

    [embedding]
    model = "all-MiniLM-L6-v2"
    batch_size = 32

    [token_reduction]
    max_tokens = 4096
    strategy = "truncate"

    [language_detection]
    min_confidence = 0.8
    detect_all = false

    [post_processors]
    remove_duplicates = true
    normalize_whitespace = true
    fix_encoding = true
    ```
