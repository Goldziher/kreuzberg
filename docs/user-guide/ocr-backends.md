# OCR Backends

Kreuzberg supports multiple OCR engines for text extraction from images and scanned documents. Each backend has different strengths, performance characteristics, and installation requirements.

## Supported Backends

### 1. Tesseract OCR

**Default backend** - Fast, accurate, supports 100+ languages.

```python
from kreuzberg import extract_file, ExtractionConfig, TesseractConfig, PSMMode

result = await extract_file(
    "document.pdf",
    config=ExtractionConfig(
        ocr=TesseractConfig(
            language="eng+deu",
            psm=PSMMode.SINGLE_BLOCK,
            output_format="markdown",
        ),
    ),
)
```

**Installation:**

```bash
# Ubuntu/Debian
sudo apt-get install tesseract-ocr tesseract-ocr-eng

# macOS
brew install tesseract tesseract-lang

# Windows
# Download from https://github.com/UB-Mannheim/tesseract/wiki
```

**Key Features:**

- 100+ languages supported
- Page segmentation modes for different layouts
- Multiple output formats (text, markdown, hOCR, TSV)
- Table detection with TSV output
- Fast and lightweight

**Best For:**

- General-purpose OCR
- Printed text
- Multi-language documents
- Production environments

### 2. EasyOCR

**Optional backend** - Better for scene text and handwriting.

```python
from kreuzberg import extract_file, ExtractionConfig, EasyOCRConfig

result = await extract_file(
    "scene_text.jpg",
    config=ExtractionConfig(
        ocr=EasyOCRConfig(
            language=("en", "de"),  # Must be tuple
            device="cuda",  # or "cpu", "mps"
            confidence_threshold=0.5,
        ),
    ),
)
```

**Installation:**

```bash
pip install "kreuzberg[easyocr]"
```

**Key Features:**

- 80+ languages supported
- GPU acceleration (CUDA, MPS)
- Confidence scores
- Better for scene text and handwriting
- Batch processing support

**Best For:**

- Scene text (photos, signs, natural environments)
- Handwritten text
- Low-quality or degraded images
- Non-Latin scripts

### 3. PaddleOCR

**Optional backend** - Excellent for Chinese and Asian languages.

```python
from kreuzberg import extract_file, ExtractionConfig, PaddleOCRConfig

result = await extract_file(
    "chinese_document.jpg",
    config=ExtractionConfig(
        ocr=PaddleOCRConfig(
            language="ch",  # ch, en, french, german, japan, korean
            device="cuda",  # or "cpu"
        ),
    ),
)
```

**Installation:**

```bash
pip install "kreuzberg[paddleocr]"
```

**Key Features:**

- Optimized for Chinese, Japanese, Korean
- GPU acceleration
- High accuracy for Asian languages
- Lightweight models

**Best For:**

- Chinese documents
- Japanese documents
- Korean documents
- Mixed CJK content

### 4. No OCR

Disable OCR completely for text-based documents:

```python
result = await extract_file(
    "text_document.pdf",
    config=ExtractionConfig(ocr=None),  # No OCR overhead
)
```

**Use When:**

- Processing only text-based (searchable) PDFs
- Minimizing processing time
- Reducing resource usage

## Choosing the Right Backend

### Decision Matrix

| Use Case                    | Recommended Backend | Why                                              |
| --------------------------- | ------------------- | ------------------------------------------------ |
| General printed text        | Tesseract           | Fast, accurate, well-supported                   |
| Scene text (photos)         | EasyOCR             | Better detection of text in natural environments |
| Handwritten text            | EasyOCR             | Better recognition of handwriting                |
| Chinese/Japanese/Korean     | PaddleOCR           | Optimized for CJK characters                     |
| Multi-language (100+ langs) | Tesseract           | Widest language support                          |
| GPU acceleration needed     | EasyOCR/PaddleOCR   | Native CUDA/MPS support                          |
| Production (speed critical) | Tesseract           | Fastest, most stable                             |
| Text-based PDFs only        | None                | No OCR overhead                                  |

### Performance Comparison

**Speed** (fastest to slowest):

1. None (no OCR)
1. Tesseract
1. PaddleOCR (with GPU)
1. EasyOCR (with GPU)
1. PaddleOCR (CPU)
1. EasyOCR (CPU)

**Accuracy** (for specific use cases):

- **Printed text**: Tesseract ≈ EasyOCR > PaddleOCR
- **Scene text**: EasyOCR > Tesseract > PaddleOCR
- **Handwriting**: EasyOCR > PaddleOCR > Tesseract
- **CJK languages**: PaddleOCR > EasyOCR > Tesseract

### Resource Requirements

| Backend   | RAM Usage | GPU Support | Model Size | Installation |
| --------- | --------- | ----------- | ---------- | ------------ |
| Tesseract | Low (MB)  | No          | ~5-10MB    | System       |
| EasyOCR   | High (GB) | CUDA/MPS    | ~100-500MB | pip          |
| PaddleOCR | Medium    | CUDA        | ~10-50MB   | pip          |

## Configuration Examples

### Default Configuration (Tesseract)

```python
# Automatically uses Tesseract if no OCR backend specified
config = ExtractionConfig()
```

### Multi-Language Document

```python
# Tesseract with multiple languages
config = ExtractionConfig(
    ocr=TesseractConfig(language="eng+deu+fra"),
)
```

### GPU-Accelerated Processing

```python
# EasyOCR with GPU
config = ExtractionConfig(
    ocr=EasyOCRConfig(
        language=("en",),
        device="cuda",  # or "mps" for Apple Silicon
    ),
)
```

### High-Accuracy Chinese OCR

```python
# PaddleOCR optimized for Chinese
config = ExtractionConfig(
    ocr=PaddleOCRConfig(
        language="ch",
        device="cuda",
    ),
)
```

### Mixed Configuration

```python
# Process with language detection, then OCR
config = ExtractionConfig(
    language_detection=LanguageDetectionConfig(),
    ocr=TesseractConfig(language="eng"),
)
```

## Installation Summary

```bash
# Basic installation (Tesseract requires separate system installation)
pip install kreuzberg
sudo apt-get install tesseract-ocr  # Ubuntu/Debian
brew install tesseract               # macOS

# With EasyOCR support
pip install "kreuzberg[easyocr]"

# With PaddleOCR support
pip install "kreuzberg[paddleocr]"

# With all OCR backends
pip install "kreuzberg[easyocr,paddleocr]"

# With all features
pip install "kreuzberg[all]"
```

## Troubleshooting

### Tesseract Not Found

```bash
# Ubuntu/Debian
sudo apt-get update && sudo apt-get install tesseract-ocr

# macOS
brew install tesseract

# Verify installation
tesseract --version
```

### EasyOCR/PaddleOCR GPU Issues

```python
# Force CPU if GPU not available
config = ExtractionConfig(
    ocr=EasyOCRConfig(
        language=("en",),
        device="cpu",  # Force CPU
    ),
)
```

### Memory Issues with EasyOCR

```python
# Reduce batch size to lower memory usage
config = ExtractionConfig(
    ocr=EasyOCRConfig(
        language=("en",),
        batch_size=1,  # Process one image at a time
    ),
)
```

## Backend Comparison

| Feature             | Tesseract | EasyOCR      | PaddleOCR              |
| ------------------- | --------- | ------------ | ---------------------- |
| **Languages**       | 100+      | 80+          | Limited (focus on CJK) |
| **Model Size**      | 5-10MB    | 100-500MB    | 10-50MB                |
| **RAM Usage**       | Low (MB)  | High (2-4GB) | Medium                 |
| **GPU Support**     | No        | CUDA/MPS     | CUDA                   |
| **Speed (CPU)**     | Fast      | Slow         | Medium                 |
| **Speed (GPU)**     | N/A       | Fast         | Fast                   |
| **Printed Text**    | Excellent | Good         | Excellent              |
| **Scene Text**      | Poor      | Excellent    | Good                   |
| **Handwriting**     | Poor      | Good         | Fair                   |
| **Complex Layouts** | Good      | Fair         | Excellent              |
| **Asian Languages** | Good      | Good         | Excellent              |

## When to Use Each Backend

### When to Use Tesseract

✅ **Recommended for:**

- General-purpose OCR
- Printed documents (invoices, reports, forms)
- Multi-language documents (100+ languages)
- Production environments
- CPU-only infrastructure
- Docker/containerized deployments
- Batch processing on standard hardware

❌ **Not ideal for:**

- Handwritten text
- Scene text (photos, signs)
- Extreme low-quality images

### When to Use EasyOCR

✅ **Recommended for:**

- Scene text extraction (photos, screenshots)
- Handwritten text
- GPU-accelerated workloads
- Real-time processing with GPU
- Low-quality or degraded images
- Mixed orientations

❌ **Not ideal for:**

- CPU-only environments (very slow)
- Memory-constrained systems
- Simple printed documents (overkill)

### When to Use PaddleOCR

✅ **Recommended for:**

- Complex document layouts
- Table-heavy documents
- Chinese/Japanese/Korean documents
- Resource-constrained environments (small models)
- High-accuracy requirements
- Asian language documents

❌ **Not ideal for:**

- Simple printed text (Tesseract is faster)
- 100+ language support (fewer languages than Tesseract)

## Best Practices

1. **Start with Tesseract** - Default backend works well for most printed documents
1. **Use EasyOCR for scene text** - Photos, screenshots, handwriting
1. **Use PaddleOCR for complex layouts** - Tables, Asian languages
1. **Enable GPU when available** - Significant speedup for EasyOCR/PaddleOCR
1. **Disable OCR for text PDFs** - Set `ocr=None` when processing searchable PDFs
1. **Match language to content** - Use correct language codes for best accuracy

## See Also

- [OCR Configuration](ocr-configuration.md) - Detailed OCR configuration options
- [Extraction Configuration](extraction-configuration.md) - Complete configuration guide
- [API Reference](../api-reference/ocr-configuration.md) - OCR configuration API reference
