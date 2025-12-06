use kreuzberg::core::config::ExtractionConfig;
use kreuzberg::core::extractor::extract_bytes;
use std::fs;

#[tokio::main]
async fn main() {
    let config = ExtractionConfig::default();
    let doc_path = "/Users/naamanhirschfeld/workspace/kreuzberg/test_documents/typst/simple.typ";
    let content = fs::read(doc_path).expect("failed to read");

    let result = extract_bytes(&content, "text/x-typst", &config).await;
    match result {
        Ok(extraction) => {
            println!("=== EXTRACTED CONTENT ===");
            println!("{}", extraction.content);
            println!("\n=== METADATA ===");
            for (k, v) in &extraction.metadata.additional {
                println!("{}: {}", k, v);
            }
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    }
}
