```rust title="Rust"
use kreuzberg::{extract_file_sync, extract_bytes_sync, ExtractionConfig, KreuzbergError};

fn main() -> kreuzberg::Result<()> {
    match extract_file_sync("document.pdf", None, &ExtractionConfig::default()) {
        Ok(result) => {
            println!("Extracted {} characters", result.content.len());
        }
        Err(KreuzbergError::Parsing { message, .. }) => {
            eprintln!("Failed to parse document: {}", message);
        }
        Err(KreuzbergError::Ocr { message, .. }) => {
            eprintln!("OCR processing failed: {}", message);
        }
        Err(KreuzbergError::MissingDependency { message, .. }) => {
            eprintln!("Missing dependency: {}", message);
        }
        Err(e) => {
            eprintln!("Extraction failed: {}", e);
        }
    }

    let pdf_bytes = b"%PDF-1.4\n...";
    match extract_bytes_sync(pdf_bytes, "application/pdf", None, &ExtractionConfig::default()) {
        Ok(result) => {
            println!("Extracted: {}", &result.content[..100.min(result.content.len())]);
            Ok(())
        }
        Err(KreuzbergError::Validation { message, .. }) => {
            eprintln!("Invalid configuration: {}", message);
            Err(KreuzbergError::Validation {
                message: message.clone(),
                source: None,
            })
        }
        Err(KreuzbergError::Ocr { message, .. }) => {
            eprintln!("OCR failed: {}", message);
            Err(KreuzbergError::Ocr {
                message: message.clone(),
                source: None,
            })
        }
        Err(e) => {
            eprintln!("Extraction failed: {}", e);
            Err(e)
        }
    }
}
```
