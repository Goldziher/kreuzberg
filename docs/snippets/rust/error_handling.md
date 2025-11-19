```rust
use kreuzberg::{extract_file_sync, ExtractionConfig, KreuzbergError};

fn main() {
    let result = extract_file_sync("document.pdf", None, &ExtractionConfig::default());

    match result {
        Ok(extraction) => {
            println!("{}", extraction.content);
        }
        Err(KreuzbergError::Validation(msg)) => {
            eprintln!("Invalid configuration: {}", msg);
        }
        Err(KreuzbergError::Parsing { message, context }) => {
            eprintln!("Failed to parse document: {}", message);
            eprintln!("Context: {:?}", context);
        }
        Err(KreuzbergError::Ocr(msg)) => {
            eprintln!("OCR processing failed: {}", msg);
        }
        Err(KreuzbergError::MissingDependency { dependency, message }) => {
            eprintln!("Missing dependency {}: {}", dependency, message);
        }
        Err(KreuzbergError::Io(e)) => {
            eprintln!("I/O error: {}", e);
        }
        Err(e) => {
            eprintln!("Extraction error: {}", e);
        }
    }
}
```
