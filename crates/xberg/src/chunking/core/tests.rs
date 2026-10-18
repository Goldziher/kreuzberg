use super::*;
use crate::XbergError;

#[test]
fn test_chunk_empty_text() {
    let config = ChunkingConfig::default();
    let result = chunk_text("", &config, None).unwrap();
    assert_eq!(result.chunks.len(), 0);
    assert_eq!(result.chunk_count, 0);
}

#[test]
fn test_chunk_short_text_single_chunk() {
    let config = ChunkingConfig {
        max_characters: 100,
        overlap: 10,
        trim: true,
        chunker_type: ChunkerType::Text,
        ..Default::default()
    };
    let text = "This is a short text.";
    let result = chunk_text(text, &config, None).unwrap();
    assert_eq!(result.chunks.len(), 1);
    assert_eq!(result.chunk_count, 1);
    assert_eq!(result.chunks[0].content, text);
}

#[test]
fn test_chunk_long_text_multiple_chunks() {
    let config = ChunkingConfig {
        max_characters: 20,
        overlap: 5,
        trim: true,
        chunker_type: ChunkerType::Text,
        ..Default::default()
    };
    let text = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let result = chunk_text(text, &config, None).unwrap();
    assert!(result.chunk_count >= 2);
    assert_eq!(result.chunks.len(), result.chunk_count);
    assert!(result.chunks.iter().all(|chunk| chunk.content.len() <= 20));
}

#[test]
fn test_chunk_text_with_overlap() {
    let config = ChunkingConfig {
        max_characters: 20,
        overlap: 5,
        trim: true,
        chunker_type: ChunkerType::Text,
        ..Default::default()
    };
    let text = "abcdefghijklmnopqrstuvwxyz0123456789";
    let result = chunk_text(text, &config, None).unwrap();
    assert!(result.chunk_count >= 2);

    if result.chunks.len() >= 2 {
        let first_chunk_end = &result.chunks[0].content[result.chunks[0].content.len().saturating_sub(5)..];
        assert!(
            result.chunks[1].content.starts_with(first_chunk_end),
            "Expected overlap '{}' at start of second chunk '{}'",
            first_chunk_end,
            result.chunks[1].content
        );
    }
}

#[cfg(feature = "embeddings")]
#[test]
fn chunk_text_applies_fast_preset_and_preserves_trim_setting() {
    const MANUAL_MAX_CHARACTERS: usize = 64;
    const FAST_PRESET_MAX_CHARACTERS: usize = 512;
    const FAST_PRESET_OVERLAP: usize = 50;

    let text = format!("  {}", "abcdefghijklmnopqrstuvwxyz".repeat(100));
    let config = ChunkingConfig {
        max_characters: MANUAL_MAX_CHARACTERS,
        overlap: 0,
        trim: false,
        preset: Some("fast".to_string()),
        ..Default::default()
    };

    let result = chunk_text(&text, &config, None).unwrap();

    assert!(result.chunks.len() > 2);
    assert_eq!(result.chunks[0].content, "  ");
    assert!(
        result
            .chunks
            .iter()
            .all(|chunk| chunk.content.len() <= FAST_PRESET_MAX_CHARACTERS)
    );

    let body_chunks: Vec<_> = result.chunks.iter().filter(|chunk| chunk.content.len() > 2).collect();
    assert_eq!(body_chunks[0].content.len(), FAST_PRESET_MAX_CHARACTERS);
    let first = &body_chunks[0].content;
    let expected_overlap = &first[first.len() - FAST_PRESET_OVERLAP..];
    assert!(body_chunks[1].content.starts_with(expected_overlap));
}

#[test]
fn test_chunk_markdown_preserves_structure() {
    let config = ChunkingConfig {
        max_characters: 50,
        overlap: 10,
        trim: true,
        chunker_type: ChunkerType::Markdown,
        ..Default::default()
    };
    let markdown = "# Title\n\nParagraph one.\n\n## Section\n\nParagraph two.";
    let result = chunk_text(markdown, &config, None).unwrap();
    assert!(result.chunk_count >= 1);
    assert!(result.chunks.iter().any(|chunk| chunk.content.contains("# Title")));
}

#[test]
fn test_chunk_markdown_with_code_blocks() {
    let config = ChunkingConfig {
        max_characters: 100,
        overlap: 10,
        trim: true,
        chunker_type: ChunkerType::Markdown,
        ..Default::default()
    };
    let markdown = "# Code Example\n\n```python\nprint('hello')\n```\n\nSome text after code.";
    let result = chunk_text(markdown, &config, None).unwrap();
    assert!(result.chunk_count >= 1);
    assert!(result.chunks.iter().any(|chunk| chunk.content.contains("```")));
}

#[test]
fn test_chunk_markdown_with_links() {
    let config = ChunkingConfig {
        max_characters: 80,
        overlap: 10,
        trim: true,
        chunker_type: ChunkerType::Markdown,
        ..Default::default()
    };
    let markdown = "Check out [this link](https://example.com) for more info.";
    let result = chunk_text(markdown, &config, None).unwrap();
    assert_eq!(result.chunk_count, 1);
    assert!(result.chunks[0].content.contains("[this link]"));
}

#[test]
fn test_chunk_text_with_trim() {
    let config = ChunkingConfig {
        max_characters: 30,
        overlap: 5,
        trim: true,
        chunker_type: ChunkerType::Text,
        ..Default::default()
    };
    let text = "  Leading and trailing spaces  should be trimmed  ";
    let result = chunk_text(text, &config, None).unwrap();
    assert!(result.chunk_count >= 1);
    assert!(result.chunks.iter().all(|chunk| !chunk.content.starts_with(' ')));
}

#[test]
fn test_chunk_text_without_trim() {
    let config = ChunkingConfig {
        max_characters: 30,
        overlap: 5,
        trim: false,
        chunker_type: ChunkerType::Text,
        ..Default::default()
    };
    let text = "  Text with spaces  ";
    let result = chunk_text(text, &config, None).unwrap();
    assert_eq!(result.chunk_count, 1);
    assert!(result.chunks[0].content.starts_with(' ') || result.chunks[0].content.len() < text.len());
}

#[test]
fn test_chunk_with_invalid_overlap() {
    let config = ChunkingConfig {
        max_characters: 10,
        overlap: 20,
        trim: true,
        chunker_type: ChunkerType::Text,
        ..Default::default()
    };
    let result = chunk_text("Some text", &config, None);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, XbergError::Validation { .. }));
}

#[test]
fn test_chunk_text_with_type_text() {
    let result = chunk_text_with_type("Simple text", 50, 10, true, ChunkerType::Text).unwrap();
    assert_eq!(result.chunk_count, 1);
    assert_eq!(result.chunks[0].content, "Simple text");
}

#[test]
fn test_chunk_text_with_type_markdown() {
    let markdown = "# Header\n\nContent here.";
    let result = chunk_text_with_type(markdown, 50, 10, true, ChunkerType::Markdown).unwrap();
    assert_eq!(result.chunk_count, 1);
    assert!(result.chunks[0].content.contains("# Header"));
}

#[test]
fn test_chunk_texts_batch_empty() {
    let config = ChunkingConfig::default();
    let texts: Vec<String> = vec![];
    let results = chunk_texts_batch(&texts, &config).unwrap();
    assert_eq!(results.len(), 0);
}

#[test]
fn test_chunk_texts_batch_multiple() {
    let config = ChunkingConfig {
        max_characters: 30,
        overlap: 5,
        trim: true,
        chunker_type: ChunkerType::Text,
        ..Default::default()
    };
    let texts: Vec<String> = vec![
        "First text".to_string(),
        "Second text".to_string(),
        "Third text".to_string(),
    ];
    let results = chunk_texts_batch(&texts, &config).unwrap();
    assert_eq!(results.len(), 3);
    assert!(results.iter().all(|r| r.chunk_count >= 1));
}

#[test]
fn test_chunk_texts_batch_mixed_lengths() {
    let config = ChunkingConfig {
        max_characters: 20,
        overlap: 5,
        trim: true,
        chunker_type: ChunkerType::Text,
        ..Default::default()
    };
    let texts: Vec<String> = vec![
        "Short".to_string(),
        "This is a longer text that should be split into multiple chunks".to_string(),
        String::new(),
    ];
    let results = chunk_texts_batch(&texts, &config).unwrap();
    assert_eq!(results.len(), 3);
    assert_eq!(results[0].chunk_count, 1);
    assert!(results[1].chunk_count > 1);
    assert_eq!(results[2].chunk_count, 0);
}

#[test]
fn test_chunk_texts_batch_error_propagation() {
    let config = ChunkingConfig {
        max_characters: 10,
        overlap: 20,
        trim: true,
        chunker_type: ChunkerType::Text,
        ..Default::default()
    };
    let texts: Vec<String> = vec!["Text one".to_string(), "Text two".to_string()];
    let result = chunk_texts_batch(&texts, &config);
    assert!(result.is_err());
}

#[test]
fn test_chunking_config_default() {
    let config = ChunkingConfig::default();
    assert_eq!(config.max_characters, 1000);
    assert_eq!(config.overlap, 200);
    assert!(config.trim);
    assert_eq!(config.chunker_type, ChunkerType::Text);
}

#[test]
fn test_chunk_very_long_text() {
    let config = ChunkingConfig {
        max_characters: 100,
        overlap: 20,
        trim: true,
        chunker_type: ChunkerType::Text,
        ..Default::default()
    };
    let text = "a".repeat(1000);
    let result = chunk_text(&text, &config, None).unwrap();
    assert!(result.chunk_count >= 10);
    assert!(result.chunks.iter().all(|chunk| chunk.content.len() <= 100));
}

#[test]
fn test_chunk_text_with_newlines() {
    let config = ChunkingConfig {
        max_characters: 30,
        overlap: 5,
        trim: true,
        chunker_type: ChunkerType::Text,
        ..Default::default()
    };
    let text = "Line one\nLine two\nLine three\nLine four\nLine five";
    let result = chunk_text(text, &config, None).unwrap();
    assert!(result.chunk_count >= 1);
}

#[test]
fn test_chunk_markdown_with_lists() {
    let config = ChunkingConfig {
        max_characters: 100,
        overlap: 10,
        trim: true,
        chunker_type: ChunkerType::Markdown,
        ..Default::default()
    };
    let markdown = "# List Example\n\n- Item 1\n- Item 2\n- Item 3\n\nMore text.";
    let result = chunk_text(markdown, &config, None).unwrap();
    assert!(result.chunk_count >= 1);
    assert!(result.chunks.iter().any(|chunk| chunk.content.contains("- Item")));
}

#[test]
fn test_chunk_markdown_with_tables() {
    let config = ChunkingConfig {
        max_characters: 150,
        overlap: 10,
        trim: true,
        chunker_type: ChunkerType::Markdown,
        ..Default::default()
    };
    let markdown = "# Table\n\n| Col1 | Col2 |\n|------|------|\n| A    | B    |\n| C    | D    |";
    let result = chunk_text(markdown, &config, None).unwrap();
    assert!(result.chunk_count >= 1);
    assert!(result.chunks.iter().any(|chunk| chunk.content.contains("|")));
}

#[test]
fn test_chunk_special_characters() {
    let config = ChunkingConfig {
        max_characters: 50,
        overlap: 5,
        trim: true,
        chunker_type: ChunkerType::Text,
        ..Default::default()
    };
    let text = "Special chars: @#$%^&*()[]{}|\\<>?/~`";
    let result = chunk_text(text, &config, None).unwrap();
    assert_eq!(result.chunk_count, 1);
    assert!(result.chunks[0].content.contains("@#$%"));
}

#[test]
fn test_chunk_unicode_characters() {
    let config = ChunkingConfig {
        max_characters: 50,
        overlap: 5,
        trim: true,
        chunker_type: ChunkerType::Text,
        ..Default::default()
    };
    let text = "Unicode: 你好世界 🌍 café résumé";
    let result = chunk_text(text, &config, None).unwrap();
    assert_eq!(result.chunk_count, 1);
    assert!(result.chunks[0].content.contains("你好"));
    assert!(result.chunks[0].content.contains("🌍"));
}

#[test]
fn test_chunk_cjk_text() {
    let config = ChunkingConfig {
        max_characters: 30,
        overlap: 5,
        trim: true,
        chunker_type: ChunkerType::Text,
        ..Default::default()
    };
    let text = "日本語のテキストです。これは長い文章で、複数のチャンクに分割されるべきです。";
    let result = chunk_text(text, &config, None).unwrap();
    assert!(result.chunk_count >= 1);
}

#[test]
fn markdown_chunk_content_matches_source_span_byte_for_byte() {
    let original_document = "# Title\n\nSome text\n\n## Section\n\nMore text that will not be merged back up";
    let config = ChunkingConfig {
        max_characters: 50,
        overlap: 0,
        trim: true,
        chunker_type: ChunkerType::Markdown,
        ..Default::default()
    };

    let result = chunk_text(original_document, &config, None).unwrap();
    assert!(!result.chunks.is_empty());

    let mut checked_a_chunk_under_a_heading = false;
    for chunk in &result.chunks {
        assert_eq!(
            chunk.content,
            &original_document[chunk.metadata.byte_start..chunk.metadata.byte_end],
            "chunk.content must be byte-identical to the source span it claims to cover"
        );
        if chunk.metadata.heading_context.is_some() {
            checked_a_chunk_under_a_heading = true;
        }
    }
    assert!(
        checked_a_chunk_under_a_heading,
        "test fixture must produce at least one chunk under a heading"
    );
}

#[test]
fn render_heading_breadcrumb_reconstructs_the_expected_breadcrumb_prefixed_text() {
    use crate::types::HeadingLevel;

    let context = HeadingContext {
        headings: vec![
            HeadingLevel {
                level: 1,
                text: "Title".to_string(),
            },
            HeadingLevel {
                level: 2,
                text: "Section".to_string(),
            },
        ],
    };
    let clean_content = "## Section\n\nMore text that will not be merged back up";

    let rendered = render_heading_breadcrumb(clean_content, &context);

    assert_eq!(
        rendered, "# Title > ## Section\n\nMore text that will not be merged back up",
        "render_heading_breadcrumb must prepend the full heading path and strip the duplicated leading heading line"
    );
}

#[test]
fn should_join_heading_levels_with_the_configured_separator() {
    use crate::types::HeadingLevel;

    let context = HeadingContext {
        headings: vec![
            HeadingLevel {
                level: 1,
                text: "Guide".to_string(),
            },
            HeadingLevel {
                level: 2,
                text: "Setup".to_string(),
            },
            HeadingLevel {
                level: 3,
                text: "Prerequisites".to_string(),
            },
        ],
    };

    let rendered = render_heading_breadcrumb("Install the dependencies.", &context);

    assert_eq!(
        rendered, "# Guide > ## Setup > ### Prerequisites\n\nInstall the dependencies.",
        "each heading level must render as ATX hashes and join with ' > ', in order"
    );
}

#[test]
fn should_render_single_heading_without_a_separator() {
    use crate::types::HeadingLevel;

    let context = HeadingContext {
        headings: vec![HeadingLevel {
            level: 2,
            text: "Setup".to_string(),
        }],
    };

    let rendered = render_heading_breadcrumb("Install the dependencies.", &context);

    assert_eq!(
        rendered, "## Setup\n\nInstall the dependencies.",
        "a single heading must not have a ' > ' separator appended"
    );
}

#[test]
fn should_prepend_only_a_blank_line_when_there_are_no_headings() {
    let context = HeadingContext { headings: vec![] };

    let rendered = render_heading_breadcrumb("Install the dependencies.", &context);

    assert_eq!(
        rendered, "\n\nInstall the dependencies.",
        "no headings means no breadcrumb text, but the blank-line separator is still emitted \
         and content is passed through unstripped"
    );
}

#[test]
fn should_strip_a_leading_duplicate_of_the_deepest_heading_from_content() {
    use crate::types::HeadingLevel;

    let context = HeadingContext {
        headings: vec![
            HeadingLevel {
                level: 1,
                text: "Guide".to_string(),
            },
            HeadingLevel {
                level: 2,
                text: "Setup".to_string(),
            },
        ],
    };

    // The Markdown splitter commonly keeps the opening heading as the first
    // line of a section's first chunk; render_heading_breadcrumb must strip
    // that one leading occurrence so the heading is not duplicated.
    let rendered = render_heading_breadcrumb("## Setup\n\nInstall the dependencies.", &context);

    assert_eq!(
        rendered, "# Guide > ## Setup\n\nInstall the dependencies.",
        "a leading occurrence of the deepest heading in content must be stripped, not duplicated"
    );
}

#[test]
fn test_strip_leading_heading_basic() {
    assert_eq!(strip_leading_heading("## Section\n\nBody", 2, "Section"), "Body");
}

#[test]
fn test_strip_leading_heading_closing_atx() {
    assert_eq!(strip_leading_heading("## Section ##\n\nBody", 2, "Section"), "Body");
}

#[test]
fn test_strip_leading_heading_no_match() {
    let text = "Some paragraph text";
    assert_eq!(strip_leading_heading(text, 2, "Section"), text);
}

#[test]
fn test_strip_leading_heading_wrong_level() {
    let text = "### Section\n\nBody";
    assert_eq!(strip_leading_heading(text, 2, "Section"), text);
}

#[test]
fn test_strip_leading_heading_single_newline() {
    assert_eq!(strip_leading_heading("# Title\nBody", 1, "Title"), "Body");
}

#[test]
fn test_strip_leading_heading_no_body() {
    assert_eq!(strip_leading_heading("## Section", 2, "Section"), "");
}

#[test]
fn test_strip_leading_heading_empty_input() {
    assert_eq!(strip_leading_heading("", 2, "Section"), "");
}

#[test]
fn test_strip_leading_heading_unicode() {
    assert_eq!(
        strip_leading_heading("## Übersicht\n\nInhalt", 2, "Übersicht"),
        "Inhalt"
    );
    assert_eq!(strip_leading_heading("# 概要\n\n本文", 1, "概要"), "本文");
}

#[test]
fn test_chunk_mixed_languages() {
    let config = ChunkingConfig {
        max_characters: 40,
        overlap: 5,
        trim: true,
        chunker_type: ChunkerType::Text,
        ..Default::default()
    };
    let text = "English text mixed with 中文文本 and some français";
    let result = chunk_text(text, &config, None).unwrap();
    assert!(result.chunk_count >= 1);
}

#[test]
fn test_chunk_offset_calculation_with_overlap() {
    let config = ChunkingConfig {
        max_characters: 20,
        overlap: 5,
        trim: false,
        chunker_type: ChunkerType::Text,
        ..Default::default()
    };
    let text = "AAAAA BBBBB CCCCC DDDDD EEEEE FFFFF";
    let result = chunk_text(text, &config, None).unwrap();

    assert!(result.chunks.len() >= 2, "Expected at least 2 chunks");

    for i in 0..result.chunks.len() {
        let chunk = &result.chunks[i];
        let metadata = &chunk.metadata;

        assert_eq!(
            metadata.byte_end - metadata.byte_start,
            chunk.content.len(),
            "Chunk {} offset range doesn't match content length",
            i
        );

        assert_eq!(metadata.chunk_index, i);
        assert_eq!(metadata.total_chunks, result.chunks.len());
    }

    for i in 0..result.chunks.len() - 1 {
        let current_chunk = &result.chunks[i];
        let next_chunk = &result.chunks[i + 1];

        assert!(
            next_chunk.metadata.byte_start < current_chunk.metadata.byte_end,
            "Chunk {} and {} don't overlap: next starts at {} but current ends at {}",
            i,
            i + 1,
            next_chunk.metadata.byte_start,
            current_chunk.metadata.byte_end
        );

        let overlap_size = current_chunk.metadata.byte_end - next_chunk.metadata.byte_start;
        assert!(
            overlap_size <= config.overlap + 10,
            "Overlap between chunks {} and {} is too large: {}",
            i,
            i + 1,
            overlap_size
        );
    }
}

#[test]
fn test_chunk_offset_calculation_without_overlap() {
    let config = ChunkingConfig {
        max_characters: 20,
        overlap: 0,
        trim: false,
        chunker_type: ChunkerType::Text,
        ..Default::default()
    };
    let text = "AAAAA BBBBB CCCCC DDDDD EEEEE FFFFF";
    let result = chunk_text(text, &config, None).unwrap();

    for i in 0..result.chunks.len() - 1 {
        let current_chunk = &result.chunks[i];
        let next_chunk = &result.chunks[i + 1];

        assert!(
            next_chunk.metadata.byte_start >= current_chunk.metadata.byte_end,
            "Chunk {} and {} overlap when they shouldn't: next starts at {} but current ends at {}",
            i,
            i + 1,
            next_chunk.metadata.byte_start,
            current_chunk.metadata.byte_end
        );
    }
}

#[test]
fn test_chunk_offset_covers_full_text() {
    let config = ChunkingConfig {
        max_characters: 15,
        overlap: 3,
        trim: false,
        chunker_type: ChunkerType::Text,
        ..Default::default()
    };
    let text = "0123456789 ABCDEFGHIJ KLMNOPQRST UVWXYZ";
    let result = chunk_text(text, &config, None).unwrap();

    assert!(result.chunks.len() >= 2, "Expected multiple chunks");

    assert_eq!(
        result.chunks[0].metadata.byte_start, 0,
        "First chunk should start at position 0"
    );

    for i in 0..result.chunks.len() - 1 {
        let current_chunk = &result.chunks[i];
        let next_chunk = &result.chunks[i + 1];

        assert!(
            next_chunk.metadata.byte_start <= current_chunk.metadata.byte_end,
            "Gap detected between chunk {} (ends at {}) and chunk {} (starts at {})",
            i,
            current_chunk.metadata.byte_end,
            i + 1,
            next_chunk.metadata.byte_start
        );
    }
}

#[test]
fn test_chunk_offset_with_various_overlap_sizes() {
    for overlap in [0, 5, 10, 20] {
        let config = ChunkingConfig {
            max_characters: 30,
            overlap,
            trim: false,
            chunker_type: ChunkerType::Text,
            ..Default::default()
        };
        let text = "Word ".repeat(30);
        let result = chunk_text(&text, &config, None).unwrap();

        for chunk in &result.chunks {
            assert!(
                chunk.metadata.byte_end > chunk.metadata.byte_start,
                "Invalid offset range for overlap {}: start={}, end={}",
                overlap,
                chunk.metadata.byte_start,
                chunk.metadata.byte_end
            );
        }

        for chunk in &result.chunks {
            assert!(
                chunk.metadata.byte_start < text.len(),
                "char_start with overlap {} is out of bounds: {}",
                overlap,
                chunk.metadata.byte_start
            );
        }
    }
}

#[test]
fn test_chunk_last_chunk_offset() {
    let config = ChunkingConfig {
        max_characters: 20,
        overlap: 5,
        trim: false,
        chunker_type: ChunkerType::Text,
        ..Default::default()
    };
    let text = "AAAAA BBBBB CCCCC DDDDD EEEEE";
    let result = chunk_text(text, &config, None).unwrap();

    assert!(result.chunks.len() >= 2, "Need multiple chunks for this test");

    let last_chunk = result.chunks.last().unwrap();
    let second_to_last = &result.chunks[result.chunks.len() - 2];

    assert!(
        last_chunk.metadata.byte_start < second_to_last.metadata.byte_end,
        "Last chunk should overlap with previous chunk"
    );

    let expected_end = text.len();
    let last_chunk_covers_end =
        last_chunk.content.trim_end() == text.trim_end() || last_chunk.metadata.byte_end >= expected_end - 5;
    assert!(last_chunk_covers_end, "Last chunk should cover the end of the text");
}

#[test]
fn test_chunk_with_page_boundaries() {
    let config = ChunkingConfig {
        max_characters: 30,
        overlap: 5,
        trim: true,
        chunker_type: ChunkerType::Text,
        ..Default::default()
    };
    let text = "Page one content here. Page two starts here and continues.";

    let boundaries = vec![
        PageBoundary {
            byte_start: 0,
            byte_end: 21,
            page_number: 1,
        },
        PageBoundary {
            byte_start: 22,
            byte_end: 58,
            page_number: 2,
        },
    ];

    let result = chunk_text(text, &config, Some(&boundaries)).unwrap();
    assert!(result.chunks.len() >= 2);

    assert_eq!(result.chunks[0].metadata.first_page, Some(1));

    let last_chunk = result.chunks.last().unwrap();
    assert_eq!(last_chunk.metadata.last_page, Some(2));
}

#[test]
fn test_chunk_without_page_boundaries() {
    let config = ChunkingConfig {
        max_characters: 30,
        overlap: 5,
        trim: true,
        chunker_type: ChunkerType::Text,
        ..Default::default()
    };
    let text = "This is some test content that should be split into multiple chunks.";

    let result = chunk_text(text, &config, None).unwrap();
    assert!(result.chunks.len() >= 2);

    for chunk in &result.chunks {
        assert_eq!(chunk.metadata.first_page, None);
        assert_eq!(chunk.metadata.last_page, None);
    }
}

// Split for file-too-long (#1567): `include!` keeps these tests in one module scope. ~keep
include!("tests/part2.rs");
