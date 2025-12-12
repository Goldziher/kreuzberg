from kreuzberg import extract_file_sync, ExtractionConfig, ChunkingConfig, PageConfig

config = ExtractionConfig(
    chunking=ChunkingConfig(chunk_size=500, overlap=50),
    pages=PageConfig(extract_pages=True)
)

result = extract_file_sync("document.pdf", config=config)

if result.chunks:
    for chunk in result.chunks:
        if chunk.metadata.first_page:
            page_range = (
                f"Page {chunk.metadata.first_page}"
                if chunk.metadata.first_page == chunk.metadata.last_page
                else f"Pages {chunk.metadata.first_page}-{chunk.metadata.last_page}"
            )
            print(f"Chunk: {chunk.text[:50]}... ({page_range})")
