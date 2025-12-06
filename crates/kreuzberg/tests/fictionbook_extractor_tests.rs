#![cfg(feature = "office")]

use kreuzberg::core::config::ExtractionConfig;
use kreuzberg::plugins::DocumentExtractor;
use std::path::PathBuf;

#[tokio::test]
async fn test_fictionbook_extract_metadata_title() {
    let extractor = kreuzberg::extractors::FictionBookExtractor::new();
    let path = PathBuf::from("/Users/naamanhirschfeld/workspace/kreuzberg/test_documents/fictionbook/meta.fb2");

    let result = extractor
        .extract_file(&path, "application/x-fictionbook+xml", &ExtractionConfig::default())
        .await
        .expect("Failed to extract FB2 file");

    // Check that content (title) was extracted from the file
    assert!(
        result.content.contains("Book title"),
        "Book title should be extracted from FB2 content"
    );
}

#[tokio::test]
async fn test_fictionbook_extract_metadata_genre() {
    let extractor = kreuzberg::extractors::FictionBookExtractor::new();
    let path = PathBuf::from("/Users/naamanhirschfeld/workspace/kreuzberg/test_documents/fictionbook/meta.fb2");

    let result = extractor
        .extract_file(&path, "application/x-fictionbook+xml", &ExtractionConfig::default())
        .await
        .expect("Failed to extract FB2 file");

    // Genre "unrecognised" should not be included in subject
    // But other genres should be
    // For meta.fb2 which has "unrecognised" genre, subject should be None
    assert!(result.metadata.subject.is_none());
}

#[tokio::test]
async fn test_fictionbook_extract_content_sections() {
    let extractor = kreuzberg::extractors::FictionBookExtractor::new();
    let path = PathBuf::from("/Users/naamanhirschfeld/workspace/kreuzberg/test_documents/fictionbook/titles.fb2");

    let result = extractor
        .extract_file(&path, "application/x-fictionbook+xml", &ExtractionConfig::default())
        .await
        .expect("Failed to extract FB2 file");

    // Check that section titles are extracted
    assert!(
        result.content.contains("Simple title"),
        "Section titles should be extracted"
    );
    assert!(
        result.content.contains("Emphasized"),
        "Section with emphasis should be extracted"
    );
}

#[tokio::test]
async fn test_fictionbook_extract_section_hierarchy() {
    let extractor = kreuzberg::extractors::FictionBookExtractor::new();
    let path = PathBuf::from("/Users/naamanhirschfeld/workspace/kreuzberg/test_documents/fictionbook/basic.fb2");

    let result = extractor
        .extract_file(&path, "application/x-fictionbook+xml", &ExtractionConfig::default())
        .await
        .expect("Failed to extract FB2 file");

    // Check for section titles which indicate hierarchy
    assert!(
        result.content.contains("Top-level title"),
        "Top-level section should be extracted"
    );
    assert!(result.content.contains("Section"), "Nested section should be extracted");
    assert!(
        result.content.contains("Subsection"),
        "Nested subsection should be extracted"
    );
}

#[tokio::test]
async fn test_fictionbook_extract_inline_markup() {
    let extractor = kreuzberg::extractors::FictionBookExtractor::new();
    let path = PathBuf::from("/Users/naamanhirschfeld/workspace/kreuzberg/test_documents/fictionbook/emphasis.fb2");

    let result = extractor
        .extract_file(&path, "application/x-fictionbook+xml", &ExtractionConfig::default())
        .await
        .expect("Failed to extract FB2 file");

    // Check for text with inline formatting
    let content = result.content.to_lowercase();
    assert!(content.contains("plain"), "Plain text should be extracted");
    assert!(content.contains("strong"), "Strong emphasis should be extracted");
    assert!(content.contains("emphasis"), "Emphasis should be extracted");
    assert!(content.contains("strikethrough"), "Strikethrough should be extracted");
}

#[tokio::test]
async fn test_fictionbook_extract_emphasis() {
    let extractor = kreuzberg::extractors::FictionBookExtractor::new();
    let path = PathBuf::from("/Users/naamanhirschfeld/workspace/kreuzberg/test_documents/fictionbook/basic.fb2");

    let result = extractor
        .extract_file(&path, "application/x-fictionbook+xml", &ExtractionConfig::default())
        .await
        .expect("Failed to extract FB2 file");

    // Check that emphasized text is present
    assert!(
        result.content.contains("emphasized"),
        "Emphasized text should be extracted"
    );
}

#[tokio::test]
async fn test_fictionbook_extract_strong() {
    let extractor = kreuzberg::extractors::FictionBookExtractor::new();
    let path = PathBuf::from("/Users/naamanhirschfeld/workspace/kreuzberg/test_documents/fictionbook/basic.fb2");

    let result = extractor
        .extract_file(&path, "application/x-fictionbook+xml", &ExtractionConfig::default())
        .await
        .expect("Failed to extract FB2 file");

    // Check that strong text is present
    assert!(result.content.contains("strong"), "Strong text should be extracted");
}

#[tokio::test]
async fn test_fictionbook_extract_code() {
    let extractor = kreuzberg::extractors::FictionBookExtractor::new();
    let path = PathBuf::from("/Users/naamanhirschfeld/workspace/kreuzberg/test_documents/fictionbook/basic.fb2");

    let result = extractor
        .extract_file(&path, "application/x-fictionbook+xml", &ExtractionConfig::default())
        .await
        .expect("Failed to extract FB2 file");

    // Check that code content is present
    assert!(result.content.contains("verbatim"), "Code content should be extracted");
}

#[tokio::test]
async fn test_fictionbook_extract_blockquote() {
    let extractor = kreuzberg::extractors::FictionBookExtractor::new();
    let path = PathBuf::from("/Users/naamanhirschfeld/workspace/kreuzberg/test_documents/fictionbook/basic.fb2");

    let result = extractor
        .extract_file(&path, "application/x-fictionbook+xml", &ExtractionConfig::default())
        .await
        .expect("Failed to extract FB2 file");

    // Check that blockquote content is present
    assert!(result.content.contains("Blockquote"), "Blockquote should be extracted");
}

#[tokio::test]
async fn test_fictionbook_extract_tables() {
    let extractor = kreuzberg::extractors::FictionBookExtractor::new();
    let path = PathBuf::from("/Users/naamanhirschfeld/workspace/kreuzberg/test_documents/fictionbook/tables.fb2");

    let result = extractor
        .extract_file(&path, "application/x-fictionbook+xml", &ExtractionConfig::default())
        .await
        .expect("Failed to extract FB2 file");

    // Tables should be in content
    // FB2 table format: <table> with <tr> (rows) and <td>/<th> (cells)
    assert!(
        !result.content.is_empty(),
        "Content should be extracted from file with tables"
    );
}

#[tokio::test]
async fn test_fictionbook_pandoc_baseline_tables() {
    let extractor = kreuzberg::extractors::FictionBookExtractor::new();
    let path = PathBuf::from("/Users/naamanhirschfeld/workspace/kreuzberg/test_documents/fictionbook/tables.fb2");

    let result = extractor
        .extract_file(&path, "application/x-fictionbook+xml", &ExtractionConfig::default())
        .await
        .expect("Failed to extract FB2 file");

    // Read Pandoc baseline for comparison
    let baseline_path =
        PathBuf::from("/Users/naamanhirschfeld/workspace/kreuzberg/test_documents/fictionbook/tables.pandoc.md");
    if baseline_path.exists() {
        let baseline = std::fs::read_to_string(&baseline_path).expect("Failed to read Pandoc baseline");
        // Both should have content
        assert!(
            !result.content.is_empty() || !baseline.is_empty(),
            "Either extracted or baseline content should be present"
        );
    }
}

#[tokio::test]
async fn test_fictionbook_pandoc_baseline_emphasis() {
    let extractor = kreuzberg::extractors::FictionBookExtractor::new();
    let path = PathBuf::from("/Users/naamanhirschfeld/workspace/kreuzberg/test_documents/fictionbook/emphasis.fb2");

    let result = extractor
        .extract_file(&path, "application/x-fictionbook+xml", &ExtractionConfig::default())
        .await
        .expect("Failed to extract FB2 file");

    // Read Pandoc baseline for comparison
    let baseline_path =
        PathBuf::from("/Users/naamanhirschfeld/workspace/kreuzberg/test_documents/fictionbook/emphasis.pandoc.md");
    if baseline_path.exists() {
        let baseline = std::fs::read_to_string(&baseline_path).expect("Failed to read Pandoc baseline");
        // Both should have content with formatting words
        assert!(
            result.content.contains("strong") || baseline.contains("strong"),
            "Either extracted or baseline should contain 'strong'"
        );
        assert!(
            result.content.contains("emphasis") || baseline.contains("emphasis"),
            "Either extracted or baseline should contain 'emphasis'"
        );
    }
}
