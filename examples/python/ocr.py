"""
OCR Extraction Example

Demonstrates OCR extraction from scanned PDFs and images.
"""

from kreuzberg import extract_file_sync, ExtractionConfig, OcrConfig


def main():
    # Basic OCR extraction
    print("=== Basic OCR ===")
    config = ExtractionConfig(
        ocr=OcrConfig(
            backend="tesseract",  # Default backend
            language="eng",  # English
        )
    )

    result = extract_file_sync("scanned_document.pdf", config=config)
    print(f"Extracted: {len(result.content)} characters")
    print(f"First 200 chars: {result.content[:200]}...")

    # OCR with custom language
    print("\n=== OCR with German ===")
    config = ExtractionConfig(
        ocr=OcrConfig(
            backend="tesseract",
            language="deu",  # German
        )
    )

    result = extract_file_sync("german_document.pdf", config=config)
    print(f"Extracted German text: {len(result.content)} characters")

    # Force OCR even for text-based PDFs
    print("\n=== Force OCR ===")
    config = ExtractionConfig(
        ocr=OcrConfig(backend="tesseract", language="eng"),
        force_ocr=True,  # Extract images and run OCR even if PDF has text
    )

    result = extract_file_sync("mixed_document.pdf", config=config)
    print(f"Forced OCR extraction: {len(result.content)} characters")

    # OCR from image
    print("\n=== OCR from Image ===")
    config = ExtractionConfig(ocr=OcrConfig(backend="tesseract", language="eng"))

    result = extract_file_sync("screenshot.png", config=config)
    print(f"Extracted from image: {len(result.content)} characters")

    # Check OCR metadata
    if result.metadata.ocr:
        print(f"OCR Language: {result.metadata.ocr.language}")
        print(f"Table Count: {result.metadata.ocr.table_count}")

    # Extract tables from OCR
    print("\n=== OCR Table Extraction ===")
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
    print(f"Found {len(result.tables)} tables")

    for i, table in enumerate(result.tables):
        print(f"\nTable {i + 1}:")
        print(f"  Rows: {len(table.cells)}")
        print(f"  Columns: {len(table.cells[0]) if table.cells else 0}")
        print(f"  Markdown:\n{table.markdown[:200]}...")


if __name__ == "__main__":
    main()
