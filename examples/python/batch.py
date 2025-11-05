"""Batch Processing Example.

Demonstrates efficient batch processing of multiple documents.
"""

import asyncio
import contextlib
from pathlib import Path

from kreuzberg import ExtractionConfig, batch_extract_files, batch_extract_files_sync


def main() -> None:
    # Synchronous batch processing
    files = [
        "document1.pdf",
        "document2.docx",
        "document3.txt",
        "document4.html",
    ]

    results = batch_extract_files_sync(files)

    for file, _result in zip(files, results, strict=False):
        pass

    # Async batch processing - better for large datasets

    async def process_batch():
        files = [f"doc{i}.pdf" for i in range(10)]
        results = await batch_extract_files(files)

        sum(len(r.content) for r in results)

        return results

    asyncio.run(process_batch())

    # Batch with configuration
    config = ExtractionConfig(
        enable_quality_processing=True,
        use_cache=True,
        ocr=None,  # Disable OCR for faster processing
    )

    results = batch_extract_files_sync(files, config=config)

    # Process directory of files
    from glob import glob

    pdf_files = glob("data/*.pdf")
    if pdf_files:
        results = batch_extract_files_sync(pdf_files[:5])  # Process first 5

        for file, _result in zip(pdf_files[:5], results, strict=False):
            pass  # Process each result as needed

    # Batch extract from bytes
    from kreuzberg import batch_extract_bytes_sync

    data_list = []
    mime_types = []

    for file in files[:3]:
        with open(file, "rb") as f:
            data_list.append(f.read())

        # Detect MIME type from extension
        ext = Path(file).suffix.lower()
        mime_map = {
            ".pdf": "application/pdf",
            ".docx": "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            ".txt": "text/plain",
            ".html": "text/html",
        }
        mime_types.append(mime_map.get(ext, "application/octet-stream"))

    results = batch_extract_bytes_sync(data_list, mime_types)

    # Error handling in batch processing
    files_with_invalid = [
        "valid1.pdf",
        "nonexistent.pdf",  # This will fail
        "valid2.txt",
    ]

    with contextlib.suppress(Exception):
        results = batch_extract_files_sync(files_with_invalid)

    # Individual processing with error handling
    for file in files_with_invalid:
        with contextlib.suppress(Exception):
            batch_extract_files_sync([file])[0]


if __name__ == "__main__":
    main()
