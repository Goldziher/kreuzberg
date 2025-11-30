# Automatic Document Classification

Kreuzberg can automatically classify documents into common types like invoices, contracts, and receipts. This allows you to build custom processing pipelines tailored to each document type.

## Installation

Document classification requires the `document-classification` extra to be installed:

```bash
pip install "kreuzberg[document-classification]"
```

## Enabling Document Classification

Document classification is disabled by default. To enable this feature, set `auto_detect_document_type=True` in your `ExtractionConfig`:

```python
from kreuzberg import ExtractionConfig, extract_file

config = ExtractionConfig(auto_detect_document_type=True)
result = await extract_file("path/to/your/document.pdf", config=config)

if result.document_type:
    print(f"Detected document type: {result.document_type}")
    print(f"Confidence: {result.document_type_confidence:.2f}")
```

## Classification Modes

You can choose between two classification modes using the `document_classification_mode` parameter in `ExtractionConfig`:

- `"text"` (default): This mode uses a rule-based classifier that analyzes the extracted text for keywords and patterns. It's fast and works well for text-based documents.
- `"vision"`: This mode uses layout information from OCR to identify document types. It's more accurate for scanned documents and images, but it requires the Tesseract OCR backend.

Here's how to use the vision-based classifier:

```python
config = ExtractionConfig(
    auto_detect_document_type=True,
    document_classification_mode="vision",
    force_ocr=True,
)
```

## Confidence Threshold

You can control the minimum confidence required for a classification to be considered valid by setting the `type_confidence_threshold` in `ExtractionConfig`. The default value is `0.7`.

```python
config = ExtractionConfig(
    auto_detect_document_type=True,
    type_confidence_threshold=0.85,  # Require 85% confidence
)
```

## Output

The classification results are available in the `ExtractionResult` object:

- `document_type`: The detected document type (e.g., `"invoice"`, `"contract"`) or `None` if no type was detected with sufficient confidence.
- `type_confidence`: The confidence score of the detection (a float between 0.0 and 1.0) or `None`.
