"""
Batch Processing Example

Demonstrates efficient batch processing of multiple documents.
"""

from kreuzberg import batch_extract_files, batch_extract_files_sync, ExtractionConfig
import asyncio
from pathlib import Path


def main():
    # Synchronous batch processing
    print("=== Synchronous Batch Processing ===")
    files = [
        "document1.pdf",
        "document2.docx",
        "document3.txt",
        "document4.html",
    ]

    results = batch_extract_files_sync(files)

    for file, result in zip(files, results):
        print(f"\n{file}:")
        print(f"  Length: {len(result.content)} chars")
        print(f"  MIME: {result.mime_type}")
        print(f"  Preview: {result.content[:100]}...")

    # Async batch processing - better for large datasets
    print("\n=== Async Batch Processing ===")

    async def process_batch():
        files = [f"doc{i}.pdf" for i in range(10)]
        results = await batch_extract_files(files)

        total_chars = sum(len(r.content) for r in results)
        print(f"Processed {len(results)} files")
        print(f"Total characters: {total_chars}")

        return results

    asyncio.run(process_batch())

    # Batch with configuration
    print("\n=== Batch with Configuration ===")
    config = ExtractionConfig(
        enable_quality_processing=True,
        use_cache=True,
        ocr=None,  # Disable OCR for faster processing
    )

    results = batch_extract_files_sync(files, config=config)
    print(f"Processed {len(results)} files with configuration")

    # Process directory of files
    print("\n=== Process Directory ===")
    from glob import glob

    pdf_files = glob("data/*.pdf")
    if pdf_files:
        results = batch_extract_files_sync(pdf_files[:5])  # Process first 5

        for file, result in zip(pdf_files[:5], results):
            filename = Path(file).name
            print(f"{filename}: {len(result.content)} chars")

    # Batch extract from bytes
    print("\n=== Batch Extract from Bytes ===")
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
    print(f"Extracted {len(results)} documents from bytes")

    # Error handling in batch processing
    print("\n=== Batch with Error Handling ===")
    files_with_invalid = [
        "valid1.pdf",
        "nonexistent.pdf",  # This will fail
        "valid2.txt",
    ]

    try:
        results = batch_extract_files_sync(files_with_invalid)
    except Exception as e:
        print(f"Batch error: {e}")
        print("Note: Batch operations fail fast on first error")
        print("Process files individually for better error handling")

    # Individual processing with error handling
    print("\n=== Individual Processing with Error Handling ===")
    for file in files_with_invalid:
        try:
            result = batch_extract_files_sync([file])[0]
            print(f"✓ {file}: {len(result.content)} chars")
        except Exception as e:
            print(f"✗ {file}: {type(e).__name__}: {e}")


if __name__ == "__main__":
    main()
