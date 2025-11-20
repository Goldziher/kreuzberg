```rust
use kreuzberg::{ExtractionConfig, ImageExtractionConfig};

fn main() {
    let config = ExtractionConfig {
        images: Some(ImageExtractionConfig {
            extract_images: Some(true),
            target_dpi: Some(200),
            max_image_dimension: Some(2048),
            auto_adjust_dpi: Some(true),
            ..Default::default()
        }),
        ..Default::default()
    };
    println!("{:?}", config.images);
}
```
