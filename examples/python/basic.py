"""Basic Extraction Example.

Demonstrates basic document extraction with Kreuzberg.
"""

from kreuzberg import ExtractionConfig, extract_file, extract_file_sync


def main() -> None:
    # Synchronous extraction - simplest approach
    result = extract_file_sync("document.pdf")

    # With configuration
    config = ExtractionConfig(
        enable_quality_processing=True,
        use_cache=True,
    )
    result = extract_file_sync("document.pdf", config=config)

    # Async extraction - for I/O-bound workloads
    import asyncio

    async def async_extract():
        return await extract_file("document.pdf")

    asyncio.run(async_extract())

    # Extract from bytes
    from kreuzberg import extract_bytes_sync

    with open("document.pdf", "rb") as f:
        data = f.read()

    result = extract_bytes_sync(data, mime_type="application/pdf")

    # Access metadata
    result = extract_file_sync("document.pdf")
    if result.metadata.pdf:
        pass


if __name__ == "__main__":
    main()
