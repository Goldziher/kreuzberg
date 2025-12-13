```rust title="Rust"
use kreuzberg::{ExtractionConfig, PdfConfig};

fn main() {
    let config = ExtractionConfig {
        pdf_options: Some(PdfConfig {
            extract_images: Some(true),
            extract_metadata: Some(true),
            passwords: Some(vec!["password1".to_string(), "password2".to_string()]),
        }),
        ..Default::default()
    };
    println!("{:?}", config.pdf_options);
}
```
