"""Basic Extraction Example.

Demonstrates basic document extraction with Kreuzberg.
"""

from kreuzberg import ExtractionConfig, extract_file, extract_file_sync


def main() -> None:
    result = extract_file_sync("document.pdf")

    config = ExtractionConfig(
        enable_quality_processing=True,
        use_cache=True,
    )
    result = extract_file_sync("document.pdf", config=config)

    import asyncio

    async def async_extract():
        return await extract_file("document.pdf")

    asyncio.run(async_extract())

    from kreuzberg import extract_bytes_sync

    with open("document.pdf", "rb") as f:
        data = f.read()

    result = extract_bytes_sync(data, mime_type="application/pdf")

    result = extract_file_sync("document.pdf")
    if result.metadata.pdf:
        pass


if __name__ == "__main__":
    main()
