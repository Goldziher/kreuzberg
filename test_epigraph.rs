#[tokio::main]
async fn main() {
    use kreuzberg::core::config::ExtractionConfig;
    use kreuzberg::plugins::DocumentExtractor;
    use std::fs;
    use std::path::PathBuf;

    let path = PathBuf::from("/Users/naamanhirschfeld/workspace/kreuzberg/test_documents/fictionbook/epigraph.fb2");
    let extractor = kreuzberg::extractors::FictionBookExtractor::new();
    let config = ExtractionConfig::default();

    match extractor
        .extract_file(&path, "application/x-fictionbook+xml", &config)
        .await
    {
        Ok(result) => {
            println!("Extracted content:");
            println!("---");
            println!("{}", result.content);
            println!("---");
            println!("Lines: {}", result.content.lines().count());
            println!("Bytes: {}", result.content.len());
        }
        Err(e) => println!("Error: {:?}", e),
    }
}
