"""
Basic Extraction Example

Demonstrates basic document extraction with Kreuzberg.
"""

from kreuzberg import extract_file, extract_file_sync, ExtractionConfig


def main():
    # Synchronous extraction - simplest approach
    print("=== Synchronous Extraction ===")
    result = extract_file_sync("document.pdf")
    print(f"Content length: {len(result.content)} characters")
    print(f"MIME type: {result.mime_type}")
    print(f"First 200 chars: {result.content[:200]}...")

    # With configuration
    print("\n=== With Configuration ===")
    config = ExtractionConfig(
        enable_quality_processing=True,
        use_cache=True,
    )
    result = extract_file_sync("document.pdf", config=config)
    print(f"Extracted {len(result.content)} characters with quality processing")

    # Async extraction - for I/O-bound workloads
    print("\n=== Async Extraction ===")
    import asyncio

    async def async_extract():
        result = await extract_file("document.pdf")
        print(f"Async extracted: {len(result.content)} characters")
        return result

    asyncio.run(async_extract())

    # Extract from bytes
    print("\n=== Extract from Bytes ===")
    from kreuzberg import extract_bytes_sync

    with open("document.pdf", "rb") as f:
        data = f.read()

    result = extract_bytes_sync(data, mime_type="application/pdf")
    print(f"Extracted from bytes: {len(result.content)} characters")

    # Access metadata
    print("\n=== Metadata ===")
    result = extract_file_sync("document.pdf")
    if result.metadata.pdf:
        print(f"PDF Pages: {result.metadata.pdf.page_count}")
        print(f"Author: {result.metadata.pdf.author}")
        print(f"Title: {result.metadata.pdf.title}")


if __name__ == "__main__":
    main()
