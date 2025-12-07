#[tokio::test]
async fn test_debug_output() {
    let extractor = kreuzberg::extractors::DocbookExtractor::new();
    let path = std::path::PathBuf::from(
        "/Users/naamanhirschfeld/workspace/kreuzberg/test_documents/docbook/docbook-chapter.docbook",
    );
    let config = kreuzberg::core::config::ExtractionConfig::default();

    let result = extractor.extract_file(&path, "application/docbook+xml", &config).await;
    if let Ok(result) = result {
        println!("\n=== EXTRACTED CONTENT (first 1000 chars) ===");
        println!("{}", &result.content[..std::cmp::min(1000, result.content.len())]);

        let h1 = result.content.matches("\n# ").count();
        let h2 = result.content.matches("\n## ").count();
        let h3 = result.content.matches("\n### ").count();
        let h4 = result.content.matches("\n#### ").count();
        println!("\n=== COUNTS ===");
        println!("H1: {}, H2: {}, H3: {}, H4: {}", h1, h2, h3, h4);

        panic!("Debug info printed");
    }
}
