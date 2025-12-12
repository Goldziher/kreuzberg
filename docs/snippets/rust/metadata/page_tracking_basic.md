use kreuzberg::{extract_file_sync, ExtractionConfig, PageConfig};

let config = ExtractionConfig {
    pages: Some(PageConfig {
        extract_pages: true,
        ..Default::default()
    }),
    ..Default::default()
};

let result = extract_file_sync("document.pdf", &config)?;

if let Some(pages) = result.pages {
    for page in pages {
        println!("Page {}:", page.page_number);
        println!("  Content: {} chars", page.content.len());
        println!("  Tables: {}", page.tables.len());
        println!("  Images: {}", page.images.len());
    }
}
