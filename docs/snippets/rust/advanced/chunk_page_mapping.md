use kreuzberg::{extract_file_sync, ExtractionConfig, ChunkingConfig, PageConfig};

let config = ExtractionConfig {
    chunking: Some(ChunkingConfig {
        chunk_size: 500,
        overlap: 50,
        ..Default::default()
    }),
    pages: Some(PageConfig {
        extract_pages: true,
        ..Default::default()
    }),
    ..Default::default()
};

let result = extract_file_sync("document.pdf", &config)?;

if let Some(chunks) = result.chunks {
    for chunk in chunks {
        if let (Some(first), Some(last)) = (chunk.metadata.first_page, chunk.metadata.last_page) {
            let page_range = if first == last {
                format!("Page {}", first)
            } else {
                format!("Pages {}-{}", first, last)
            };
            println!("Chunk: {}... ({})", &chunk.text[..50], page_range);
        }
    }
}
