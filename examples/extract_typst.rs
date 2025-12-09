use kreuzberg::core::config::ExtractionConfig;
use kreuzberg::core::extractor::extract_bytes;
use std::{env, fs, path::PathBuf};

fn default_typst_fixture() -> PathBuf {
    PathBuf::from("test_documents/typst/simple.typ")
}

#[tokio::main]
async fn main() {
    let config = ExtractionConfig::default();
    let doc_path = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(default_typst_fixture);
    let content = fs::read(&doc_path).unwrap_or_else(|err| panic!("failed to read {}: {}", doc_path.display(), err));

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
