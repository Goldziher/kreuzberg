use kreuzberg::extract_file_sync;

let result = extract_file_sync("document.pdf", &Default::default())?;

if let Some(pages) = result.metadata.pages {
    if let Some(boundaries) = pages.boundaries {
        for boundary in boundaries.iter().take(3) {
            let page_text = &result.content[boundary.byte_start..boundary.byte_end];

            println!("Page {}:", boundary.page_number);
            println!("  Byte range: {}-{}", boundary.byte_start, boundary.byte_end);
            println!("  Preview: {}...", &page_text[..100.min(page_text.len())]);
        }
    }
}
