# Types

Core data structures for extraction results, configuration, and metadata.

## ExtractionResult

The result of a file extraction, containing the extracted text, MIME type, metadata, and table data:

::: kreuzberg.ExtractionResult

## ExtractionConfig

Configuration options for extraction functions:

::: kreuzberg.ExtractionConfig

## TableData

A TypedDict that contains data extracted from tables in documents:

::: kreuzberg.TableData

## Image Extraction Types

### ExtractedImage

Represents an image extracted from a document:

::: kreuzberg.ExtractedImage

### ImageOCRResult

Contains the result of running OCR on an extracted image:

::: kreuzberg.ImageOCRResult

### ImageExtractionConfig

Configuration for extracting images from documents:

::: kreuzberg.ImageExtractionConfig

## OCR Configuration

### TesseractConfig

::: kreuzberg.TesseractConfig

### EasyOCRConfig

::: kreuzberg.EasyOCRConfig

### PaddleOCRConfig

::: kreuzberg.PaddleOCRConfig

## Table Extraction Configuration

Configuration options for table extraction (vision-based and OCR-based):

::: kreuzberg.TableExtractionConfig

## Chunking Configuration

Configuration for text chunking:

::: kreuzberg.ChunkingConfig

## Keyword Extraction Configuration

Configuration for keyword extraction using KeyBERT:

::: kreuzberg.KeywordExtractionConfig

## Entity Extraction Configuration

Configuration for entity extraction using spaCy:

::: kreuzberg.EntityExtractionConfig

## Language Detection Configuration

Configuration options for automatic language detection:

::: kreuzberg.LanguageDetectionConfig

## JSON Extraction Configuration

Configuration for enhanced JSON document processing:

::: kreuzberg.JSONExtractionConfig

## HTML to Markdown Configuration

Configuration options for converting HTML content to Markdown:

::: kreuzberg.HTMLToMarkdownConfig

## Token Reduction Configuration

Configuration options for token reduction and text optimization:

::: kreuzberg.TokenReductionConfig

## PSMMode (Page Segmentation Mode)

::: kreuzberg.PSMMode

## Entity

Represents an extracted named entity:

::: kreuzberg.Entity

## Metadata

A TypedDict that contains optional metadata fields extracted from documents:

::: kreuzberg.Metadata

## OutputFormatType

The output format for Tesseract OCR processing:

```python
OutputFormatType = Literal["text", "tsv", "hocr", "markdown"]
```

- `markdown` (default): Structured markdown output with preserved formatting
- `text`: Plain text, fastest option
- `tsv`: Tab-separated values with word positions and confidence scores
- `hocr`: HTML-based OCR format with detailed position information
