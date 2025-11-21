```rust
use kreuzberg::{extract_file, ExtractionConfig};

let config = ExtractionConfig {
    enable_quality_processing: true,
    ..Default::default()
};
let result = extract_file("scanned_document.pdf", None, &config).await?;

if let Some(quality) = result.metadata.additional.get("quality_score") {
    let score: f64 = quality.as_f64().unwrap_or(0.0);
    if score < 0.5 {
        println!("Warning: Low quality extraction ({:.2})", score);
    } else {
        println!("Quality score: {:.2}", score);
    }
}
```
