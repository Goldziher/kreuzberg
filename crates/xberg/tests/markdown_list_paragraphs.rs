#![cfg(feature = "chunking")]

mod helpers;

use helpers::extract_bytes_document;
use xberg::{ChunkerType, ChunkingConfig, ExtractionConfig};

#[tokio::test]
async fn should_keep_paragraph_breaks_in_markdown_list_item_chunks() {
    let config = ExtractionConfig {
        use_cache: false,
        chunking: Some(ChunkingConfig {
            max_characters: 1000,
            overlap: 0,
            chunker_type: ChunkerType::Markdown,
            ..Default::default()
        }),
        ..Default::default()
    };

    for markdown in [
        "- first paragraph\n\n  second paragraph\n",
        "1. first paragraph\n\n   second paragraph\n",
        "- **first paragraph**\n\n  *second paragraph*\n",
    ] {
        let result = extract_bytes_document(markdown.as_bytes(), "text/markdown", &config)
            .await
            .expect("Markdown extraction should succeed");
        let chunks = result.chunks.expect("Chunking should produce chunks");

        assert_eq!(chunks.len(), 1, "input: {markdown:?}");
        assert_eq!(
            chunks[0].content, "first paragraph\n\nsecond paragraph",
            "input: {markdown:?}"
        );
    }
}
