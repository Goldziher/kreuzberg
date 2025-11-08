"""OCR Extraction Example.

Demonstrates OCR extraction from scanned PDFs and images.
"""

from kreuzberg import ExtractionConfig, OcrConfig, extract_file_sync


def main() -> None:
    config = ExtractionConfig(
        ocr=OcrConfig(
            backend="tesseract",
            language="eng",
        )
    )

    result = extract_file_sync("scanned_document.pdf", config=config)

    config = ExtractionConfig(
        ocr=OcrConfig(
            backend="tesseract",
            language="deu",
        )
    )

    result = extract_file_sync("german_document.pdf", config=config)

    config = ExtractionConfig(
        ocr=OcrConfig(backend="tesseract", language="eng"),
        force_ocr=True,
    )

    result = extract_file_sync("mixed_document.pdf", config=config)

    config = ExtractionConfig(ocr=OcrConfig(backend="tesseract", language="eng"))

    result = extract_file_sync("screenshot.png", config=config)

    if result.metadata.ocr:
        pass

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
