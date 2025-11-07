"""OCR Extraction Example.

Demonstrates OCR extraction from scanned PDFs and images.
"""

from kreuzberg import ExtractionConfig, OcrConfig, extract_file_sync


def main() -> None:
    # Basic OCR extraction
    config = ExtractionConfig(
        ocr=OcrConfig(
            backend="tesseract",  # Default backend
            language="eng",  # English
        )
    )

    result = extract_file_sync("scanned_document.pdf", config=config)

    # OCR with custom language
    config = ExtractionConfig(
        ocr=OcrConfig(
            backend="tesseract",
            language="deu",  # German
        )
    )

    result = extract_file_sync("german_document.pdf", config=config)

    # Force OCR even for text-based PDFs
    config = ExtractionConfig(
        ocr=OcrConfig(backend="tesseract", language="eng"),
        force_ocr=True,  # Extract images and run OCR even if PDF has text
    )

    result = extract_file_sync("mixed_document.pdf", config=config)

    # OCR from image
    config = ExtractionConfig(ocr=OcrConfig(backend="tesseract", language="eng"))

    result = extract_file_sync("screenshot.png", config=config)

    # Check OCR metadata
    if result.metadata.ocr:
        pass

    # Extract tables from OCR
    from kreuzberg import TesseractConfig

    config = ExtractionConfig(
        ocr=OcrConfig(
            backend="tesseract",
            language="eng",
            tesseract_config=TesseractConfig(
                enable_table_detection=True,
            ),
        )
    )

    result = extract_file_sync("table_document.pdf", config=config)

    for _i, _table in enumerate(result.tables):
        pass


if __name__ == "__main__":
    main()
