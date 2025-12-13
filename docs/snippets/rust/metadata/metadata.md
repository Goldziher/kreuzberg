```rust title="Rust"
use kreuzberg::{extract_file_sync, ExtractionConfig};

fn main() -> kreuzberg::Result<()> {
    let result = extract_file_sync("document.pdf", None, &ExtractionConfig::default())?;

    if let Some(pdf_meta) = result.metadata.pdf {
        if let Some(pages) = pdf_meta.page_count {
            println!("Pages: {}", pages);
        }
        if let Some(author) = pdf_meta.author {
            println!("Author: {}", author);
        }
        if let Some(title) = pdf_meta.title {
            println!("Title: {}", title);
        }
    }

    let html_result = extract_file_sync("page.html", None, &ExtractionConfig::default())?;
    if let Some(html_meta) = html_result.metadata.html {
        if let Some(title) = html_meta.title {
            println!("Title: {}", title);
        }
        if let Some(desc) = html_meta.description {
            println!("Description: {}", desc);
        }
        if let Some(og_img) = html_meta.og_image {
            println!("Open Graph Image: {}", og_img);
        }
    }
    Ok(())
}
```
