#[test]
fn test_chunk_empty_boundaries() {
    let config = ChunkingConfig {
        max_characters: 30,
        overlap: 5,
        trim: true,
        chunker_type: ChunkerType::Text,
        ..Default::default()
    };
    let text = "Some text content here.";
    let boundaries: Vec<PageBoundary> = vec![];

    let result = chunk_text(text, &config, Some(&boundaries)).unwrap();
    assert_eq!(result.chunks.len(), 1);

    assert_eq!(result.chunks[0].metadata.first_page, None);
    assert_eq!(result.chunks[0].metadata.last_page, None);
}

#[test]
fn test_chunk_spanning_multiple_pages() {
    let config = ChunkingConfig {
        max_characters: 50,
        overlap: 5,
        trim: false,
        chunker_type: ChunkerType::Text,
        ..Default::default()
    };
    let text = "0123456789 AAAAAAAAAA 1111111111 BBBBBBBBBB 2222222222";

    let boundaries = vec![
        PageBoundary {
            byte_start: 0,
            byte_end: 20,
            page_number: 1,
        },
        PageBoundary {
            byte_start: 20,
            byte_end: 40,
            page_number: 2,
        },
        PageBoundary {
            byte_start: 40,
            byte_end: 54,
            page_number: 3,
        },
    ];

    let result = chunk_text(text, &config, Some(&boundaries)).unwrap();
    assert!(result.chunks.len() >= 2);

    for chunk in &result.chunks {
        assert!(chunk.metadata.first_page.is_some() || chunk.metadata.last_page.is_some());
    }
}

#[test]
fn test_chunk_text_with_invalid_boundary_range() {
    let config = ChunkingConfig {
        max_characters: 30,
        overlap: 5,
        trim: true,
        chunker_type: ChunkerType::Text,
        ..Default::default()
    };
    let text = "Page one content here. Page two content.";

    let boundaries = vec![PageBoundary {
        byte_start: 10,
        byte_end: 5,
        page_number: 1,
    }];

    let result = chunk_text(text, &config, Some(&boundaries));
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("Invalid boundary range"));
    assert!(err.to_string().contains("byte_start"));
}

#[test]
fn test_chunk_text_with_unsorted_boundaries() {
    let config = ChunkingConfig {
        max_characters: 30,
        overlap: 5,
        trim: true,
        chunker_type: ChunkerType::Text,
        ..Default::default()
    };
    let text = "Page one content here. Page two content.";

    let boundaries = vec![
        PageBoundary {
            byte_start: 22,
            byte_end: 40,
            page_number: 2,
        },
        PageBoundary {
            byte_start: 0,
            byte_end: 21,
            page_number: 1,
        },
    ];

    let result = chunk_text(text, &config, Some(&boundaries));
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("not sorted"));
    assert!(err.to_string().contains("boundaries"));
}

#[test]
fn test_chunk_text_with_overlapping_boundaries() {
    let config = ChunkingConfig {
        max_characters: 30,
        overlap: 5,
        trim: true,
        chunker_type: ChunkerType::Text,
        ..Default::default()
    };
    let text = "Page one content here. Page two content.";

    let boundaries = vec![
        PageBoundary {
            byte_start: 0,
            byte_end: 25,
            page_number: 1,
        },
        PageBoundary {
            byte_start: 20,
            byte_end: 40,
            page_number: 2,
        },
    ];

    let result = chunk_text(text, &config, Some(&boundaries));
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("Overlapping"));
    assert!(err.to_string().contains("boundaries"));
}

#[test]
fn test_chunk_with_pages_basic() {
    let config = ChunkingConfig {
        max_characters: 25,
        overlap: 5,
        trim: true,
        chunker_type: ChunkerType::Text,
        ..Default::default()
    };
    let text = "First page content here.Second page content here.Third page.";

    let boundaries = vec![
        PageBoundary {
            byte_start: 0,
            byte_end: 24,
            page_number: 1,
        },
        PageBoundary {
            byte_start: 24,
            byte_end: 50,
            page_number: 2,
        },
        PageBoundary {
            byte_start: 50,
            byte_end: 60,
            page_number: 3,
        },
    ];

    let result = chunk_text(text, &config, Some(&boundaries)).unwrap();

    if !result.chunks.is_empty() {
        assert!(result.chunks[0].metadata.first_page.is_some());
    }
}

#[test]
fn test_chunk_with_pages_single_page_chunk() {
    let config = ChunkingConfig {
        max_characters: 100,
        overlap: 10,
        trim: true,
        chunker_type: ChunkerType::Text,
        ..Default::default()
    };
    let text = "All content on single page fits in one chunk.";

    let boundaries = vec![PageBoundary {
        byte_start: 0,
        byte_end: 45,
        page_number: 1,
    }];

    let result = chunk_text(text, &config, Some(&boundaries)).unwrap();
    assert_eq!(result.chunks.len(), 1);
    assert_eq!(result.chunks[0].metadata.first_page, Some(1));
    assert_eq!(result.chunks[0].metadata.last_page, Some(1));
}

#[test]
fn test_chunk_with_pages_no_overlap() {
    let config = ChunkingConfig {
        max_characters: 20,
        overlap: 0,
        trim: false,
        chunker_type: ChunkerType::Text,
        ..Default::default()
    };
    let text = "AAAAA BBBBB CCCCC DDDDD";

    let boundaries = vec![
        PageBoundary {
            byte_start: 0,
            byte_end: 11,
            page_number: 1,
        },
        PageBoundary {
            byte_start: 11,
            byte_end: 23,
            page_number: 2,
        },
    ];

    let result = chunk_text(text, &config, Some(&boundaries)).unwrap();
    assert!(!result.chunks.is_empty());

    for chunk in &result.chunks {
        if let (Some(first), Some(last)) = (chunk.metadata.first_page, chunk.metadata.last_page) {
            assert!(first <= last);
        }
    }
}

#[test]
fn test_chunk_metadata_page_range_accuracy() {
    let config = ChunkingConfig {
        max_characters: 30,
        overlap: 5,
        trim: true,
        chunker_type: ChunkerType::Text,
        ..Default::default()
    };
    let text = "Page One Content Here.Page Two.";

    let boundaries = vec![
        PageBoundary {
            byte_start: 0,
            byte_end: 21,
            page_number: 1,
        },
        PageBoundary {
            byte_start: 21,
            byte_end: 31,
            page_number: 2,
        },
    ];

    let result = chunk_text(text, &config, Some(&boundaries)).unwrap();

    for chunk in &result.chunks {
        assert_eq!(chunk.metadata.byte_end - chunk.metadata.byte_start, chunk.content.len());
    }
}

/// Regression test for GitHub issue #439:
/// Chunk metadata reports wrong page numbers for documents with many pages.
/// The byte offset drift causes chunks near the end of the document to
/// reference pages far earlier than where their content actually resides.
#[test]
fn test_issue_439_chunk_page_metadata_many_pages() {
    let num_pages = 50;
    let mut full_text = String::new();
    let mut boundaries = Vec::new();

    for p in 1..=num_pages {
        let page_content = format!(
            "Page {} content. This is the text on page number {}. It has some words to fill space here. ",
            p, p
        );
        let start = full_text.len();
        full_text.push_str(&page_content);
        let end = full_text.len();
        boundaries.push(PageBoundary {
            byte_start: start,
            byte_end: end,
            page_number: p,
        });
    }

    let config = ChunkingConfig {
        max_characters: 200,
        overlap: 50,
        trim: true,
        chunker_type: ChunkerType::Text,
        ..Default::default()
    };

    let result = chunk_text(&full_text, &config, Some(&boundaries)).unwrap();

    let last_chunk = result.chunks.last().unwrap();
    assert!(
        last_chunk.metadata.last_page.unwrap() >= num_pages - 2,
        "Last chunk should reference near the last page ({}), but got {:?}",
        num_pages,
        last_chunk.metadata.last_page
    );

    for (i, chunk) in result.chunks.iter().enumerate() {
        let actual_pos = full_text
            .find(&chunk.content)
            .expect("Chunk content must be a substring of the original text");
        let actual_page = boundaries
            .iter()
            .find(|b| actual_pos >= b.byte_start && actual_pos < b.byte_end)
            .map(|b| b.page_number);

        if let (Some(reported), Some(actual)) = (chunk.metadata.first_page, actual_page) {
            assert_eq!(
                reported, actual,
                "Chunk {} reports first_page={} but content starts on page {} \
                 (byte_start={}, actual_pos={})",
                i, reported, actual, chunk.metadata.byte_start, actual_pos
            );
        }
    }
}

/// Verify that chunk byte_start/byte_end match the actual position of the
/// chunk content within the original text.
#[test]
fn test_issue_439_chunk_byte_offsets_match_text_position() {
    let text = "Alpha bravo charlie delta echo foxtrot golf hotel india juliet kilo lima mike november oscar papa quebec romeo sierra tango uniform victor whiskey xray yankee zulu. ";
    let repeated = text.repeat(5);

    let boundaries = vec![
        PageBoundary {
            byte_start: 0,
            byte_end: text.len(),
            page_number: 1,
        },
        PageBoundary {
            byte_start: text.len(),
            byte_end: text.len() * 2,
            page_number: 2,
        },
        PageBoundary {
            byte_start: text.len() * 2,
            byte_end: text.len() * 3,
            page_number: 3,
        },
        PageBoundary {
            byte_start: text.len() * 3,
            byte_end: text.len() * 4,
            page_number: 4,
        },
        PageBoundary {
            byte_start: text.len() * 4,
            byte_end: text.len() * 5,
            page_number: 5,
        },
    ];

    let config = ChunkingConfig {
        max_characters: 80,
        overlap: 20,
        trim: true,
        chunker_type: ChunkerType::Text,
        ..Default::default()
    };

    let result = chunk_text(&repeated, &config, Some(&boundaries)).unwrap();

    for (i, chunk) in result.chunks.iter().enumerate() {
        let byte_start = chunk.metadata.byte_start;
        let byte_end = chunk.metadata.byte_end;
        assert!(
            byte_end <= repeated.len(),
            "Chunk {} byte_end ({}) exceeds text length ({})",
            i,
            byte_end,
            repeated.len()
        );
        assert_eq!(
            &repeated[byte_start..byte_end],
            chunk.content,
            "Chunk {} content doesn't match text at byte_start={}..byte_end={}",
            i,
            byte_start,
            byte_end
        );
    }
}

#[test]
fn test_chunk_page_range_boundary_edge_cases() {
    let config = ChunkingConfig {
        max_characters: 10,
        overlap: 2,
        trim: false,
        chunker_type: ChunkerType::Text,
        ..Default::default()
    };
    let text = "0123456789ABCDEFGHIJ";

    let boundaries = vec![
        PageBoundary {
            byte_start: 0,
            byte_end: 10,
            page_number: 1,
        },
        PageBoundary {
            byte_start: 10,
            byte_end: 20,
            page_number: 2,
        },
    ];

    let result = chunk_text(text, &config, Some(&boundaries)).unwrap();

    for chunk in &result.chunks {
        let on_page1 = chunk.metadata.byte_start < 10;
        let on_page2 = chunk.metadata.byte_end > 10;

        if on_page1 && on_page2 {
            assert_eq!(chunk.metadata.first_page, Some(1));
            assert_eq!(chunk.metadata.last_page, Some(2));
        } else if on_page1 {
            assert_eq!(chunk.metadata.first_page, Some(1));
        } else if on_page2 {
            assert_eq!(chunk.metadata.first_page, Some(2));
        }
    }
}

fn make_large_table(rows: usize) -> String {
    let mut s = "| Name | Value | Description |\n|------|-------|-------------|\n".to_string();
    for i in 0..rows {
        s.push_str(&format!(
            "| item{i} | {i} | Description of item {i} with some extra text |\n"
        ));
    }
    s
}

#[test]
fn table_repeat_header_prepends_to_continuation_chunks() {
    let markdown = make_large_table(40);
    let config = ChunkingConfig {
        max_characters: 300,
        overlap: 0,
        trim: true,
        chunker_type: ChunkerType::Markdown,
        table_chunking: TableChunkingMode::RepeatHeader,
        ..Default::default()
    };
    let result = chunk_text(&markdown, &config, None).unwrap();
    assert!(result.chunks.len() > 1, "table must split into multiple chunks");

    for chunk in &result.chunks {
        let trimmed = chunk.content.trim_start();
        if trimmed.starts_with('|') {
            assert!(
                chunk.content.contains("|------|"),
                "chunk missing separator row (header must be prepended):\n{:?}",
                chunk.content,
            );
            assert!(
                chunk.content.contains("| Name | Value | Description |"),
                "chunk missing header row:\n{:?}",
                chunk.content,
            );
        }
    }
}

#[test]
fn table_split_mode_default_leaves_continuation_chunks_without_header() {
    let markdown = make_large_table(40);
    let config = ChunkingConfig {
        max_characters: 300,
        overlap: 0,
        trim: true,
        chunker_type: ChunkerType::Markdown,
        ..Default::default()
    };
    let result = chunk_text(&markdown, &config, None).unwrap();
    assert!(result.chunks.len() > 1, "table must split into multiple chunks");

    let continuation_without_header = result.chunks.iter().skip(1).any(|c| {
        let t = c.content.trim_start();
        t.starts_with('|') && !c.content.contains("|------|")
    });
    assert!(
        continuation_without_header,
        "default Split mode must not inject headers into continuation chunks"
    );
}

#[test]
fn table_repeat_header_two_split_tables_each_get_own_header() {
    let table1 = "| Name | Value | Description |\n|------|-------|-------------|\n".to_string()
        + &"| item | val | some description text here |\n".repeat(30);
    let separator = "\n\n---\n\n";
    let table2 = "| Alpha | Beta | Gamma |\n|-------|------|-------|\n".to_string()
        + &"| alpha | beta | gamma value here |\n".repeat(30);
    let markdown = format!("{table1}{separator}{table2}");

    let config = ChunkingConfig {
        max_characters: 300,
        overlap: 0,
        trim: true,
        chunker_type: ChunkerType::Markdown,
        table_chunking: TableChunkingMode::RepeatHeader,
        ..Default::default()
    };
    let result = chunk_text(&markdown, &config, None).unwrap();
    assert!(result.chunks.len() > 2, "both tables must produce multiple chunks");

    for chunk in &result.chunks {
        let trimmed = chunk.content.trim_start();
        if !trimmed.starts_with('|') {
            continue;
        }
        let mut lines = trimmed.lines();
        lines.next();
        let second = lines.next().unwrap_or("").trim();
        assert!(
            second.starts_with('|') && second.contains('-'),
            "table chunk missing separator on second line:\n{:?}",
            chunk.content,
        );
    }

    for chunk in &result.chunks {
        if chunk.content.contains("alpha") || chunk.content.contains("Alpha") {
            assert!(
                !chunk.content.contains("Name") || chunk.content.contains("Alpha"),
                "table2 chunk must not be contaminated with table1 header:\n{:?}",
                chunk.content,
            );
        }
    }
}

#[test]
fn table_repeat_header_text_chunker_is_unaffected() {
    let markdown = make_large_table(40);
    let config = ChunkingConfig {
        max_characters: 300,
        overlap: 0,
        trim: true,
        chunker_type: ChunkerType::Text,
        table_chunking: TableChunkingMode::RepeatHeader,
        ..Default::default()
    };
    let result = chunk_text(&markdown, &config, None).unwrap();
    assert!(!result.chunks.is_empty());
}

#[test]
fn table_repeat_header_single_chunk_table_unchanged() {
    let markdown = "| Col1 | Col2 |\n|------|------|\n| A    | B    |\n| C    | D    |\n";
    let config = ChunkingConfig {
        max_characters: 5000,
        overlap: 0,
        trim: true,
        chunker_type: ChunkerType::Markdown,
        table_chunking: TableChunkingMode::RepeatHeader,
        ..Default::default()
    };
    let result = chunk_text(markdown, &config, None).unwrap();
    assert_eq!(result.chunks.len(), 1, "small table must stay in one chunk");
    assert_eq!(
        result.chunks[0].content.matches("|------|").count(),
        1,
        "header must appear exactly once, not be duplicated"
    );
}

#[test]
fn table_repeat_header_with_overlap_prepends_header_to_continuation_chunks() {
    let markdown = make_large_table(40);
    let config = ChunkingConfig {
        max_characters: 300,
        overlap: 50,
        trim: true,
        chunker_type: ChunkerType::Markdown,
        table_chunking: TableChunkingMode::RepeatHeader,
        ..Default::default()
    };
    let result = chunk_text(&markdown, &config, None).unwrap();
    assert!(
        result.chunks.len() > 2,
        "table must split into multiple chunks with overlap"
    );
    for chunk in &result.chunks {
        let lines: Vec<&str> = chunk.content.lines().filter(|l| !l.is_empty()).collect();
        if lines.first().is_some_and(|l| l.starts_with('|')) {
            let second_line_is_separator = lines
                .get(1)
                .is_some_and(|l| l.chars().all(|c| matches!(c, '|' | '-' | ' ')));
            assert!(
                second_line_is_separator,
                "table chunk must have separator on 2nd line; got:\n{}",
                chunk.content
            );
        }
    }
}

#[test]
fn table_repeat_header_oversized_header_does_not_panic() {
    let wide_cols: String = (0..20).map(|i| format!("| Column {i:02} ")).collect::<String>() + "|";
    let separator: String = (0..20).map(|_| "|----------").collect::<String>() + "|";
    let row: String = (0..20).map(|i| format!("| value  {i:02} ")).collect::<String>() + "|";
    let rows: String = (0..20).map(|_| format!("{row}\n")).collect::<String>();
    let markdown = format!("{wide_cols}\n{separator}\n{rows}");
    let config = ChunkingConfig {
        max_characters: 50,
        overlap: 0,
        trim: true,
        chunker_type: ChunkerType::Markdown,
        table_chunking: TableChunkingMode::RepeatHeader,
        ..Default::default()
    };
    let result = chunk_text(&markdown, &config, None).unwrap();
    assert!(
        !result.chunks.is_empty(),
        "must produce chunks even with oversized header"
    );
}

/// A data row whose cell contains a lone dash (`| - |`) must NOT be treated
/// as a separator row and must NOT trigger spurious header injection.
#[test]
fn table_repeat_header_single_dash_data_cell_not_misidentified_as_separator() {
    let markdown = "| Item | Status |\n|------|--------|\n| foo  | - |\n| bar  | done |\n| baz  | :- |\n";
    let config = ChunkingConfig {
        max_characters: 5000,
        overlap: 0,
        trim: true,
        chunker_type: ChunkerType::Markdown,
        table_chunking: TableChunkingMode::RepeatHeader,
        ..Default::default()
    };
    let result = chunk_text(markdown, &config, None).unwrap();
    assert_eq!(result.chunks.len(), 1, "small table must fit in one chunk");
    assert_eq!(
        result.chunks[0].content.matches("|------|").count(),
        1,
        "separator must appear exactly once — no spurious injection from data rows with dashes:\n{:?}",
        result.chunks[0].content,
    );
    assert!(
        result.chunks[0].content.contains("| foo  | - |"),
        "data row with lone-dash cell must be preserved:\n{:?}",
        result.chunks[0].content,
    );
    assert!(
        result.chunks[0].content.contains("| baz  | :- |"),
        "data row with colon-dash cell must be preserved:\n{:?}",
        result.chunks[0].content,
    );
}

/// A CRLF-input table whose rows are split across chunks must produce continuation
/// chunks where the injected header uses `\r\n` consistently with the body.
#[test]
fn table_repeat_header_crlf_input_header_uses_crlf_line_endings() {
    let header = "| Name | Value |\r\n|------|-------|\r\n";
    let data_rows: String = (0..40).map(|i| format!("| item{i:02} | {i:03} |\r\n")).collect();
    let markdown = format!("{header}{data_rows}");

    let config = ChunkingConfig {
        max_characters: 200,
        overlap: 0,
        trim: true,
        chunker_type: ChunkerType::Markdown,
        table_chunking: TableChunkingMode::RepeatHeader,
        ..Default::default()
    };
    let result = chunk_text(&markdown, &config, None).unwrap();
    assert!(result.chunks.len() > 1, "CRLF table must split into multiple chunks");

    for (i, chunk) in result.chunks.iter().enumerate().skip(1) {
        let trimmed = chunk.content.trim_start();
        if !trimmed.starts_with('|') {
            continue;
        }
        assert!(
            chunk.content.contains("\r\n"),
            "continuation chunk {} must contain \\r\\n line endings from injected header, but got:\n{:?}",
            i,
            chunk.content,
        );
        assert!(
            chunk.content.contains("| Name | Value |\r\n"),
            "continuation chunk {} injected header row must end with \\r\\n, not \\n:\n{:?}",
            i,
            chunk.content,
        );
        assert!(
            chunk.content.contains("|-------|\r\n"),
            "continuation chunk {} injected separator must end with \\r\\n, not \\n:\n{:?}",
            i,
            chunk.content,
        );
    }
}

/// Tables using GFM alignment separators (`:---`, `---:`, `:---:`) must still be
/// correctly detected and their headers injected into continuation chunks.
#[test]
fn table_repeat_header_alignment_separators_are_detected_correctly() {
    let header = "| Left | Right | Center |\n|:-----|------:|:------:|\n";
    let data_rows: String = (0..40).map(|i| format!("| l{i:02} | r{i:02} | c{i:02} |\n")).collect();
    let markdown = format!("{header}{data_rows}");

    let config = ChunkingConfig {
        max_characters: 200,
        overlap: 0,
        trim: true,
        chunker_type: ChunkerType::Markdown,
        table_chunking: TableChunkingMode::RepeatHeader,
        ..Default::default()
    };
    let result = chunk_text(&markdown, &config, None).unwrap();
    assert!(
        result.chunks.len() > 1,
        "alignment-separator table must split into multiple chunks"
    );

    for chunk in &result.chunks {
        let trimmed = chunk.content.trim_start();
        if !trimmed.starts_with('|') {
            continue;
        }
        assert!(
            chunk.content.contains("| Left | Right | Center |"),
            "alignment-separator table chunk missing header row:\n{:?}",
            chunk.content,
        );
        assert!(
            chunk.content.contains("|:-----|------:|:------:|"),
            "alignment-separator table chunk missing alignment separator row:\n{:?}",
            chunk.content,
        );
    }
}

/// Regression test for #255: with `ChunkSizing::Tokenizer` pointed at a registered
/// [`crate::plugins::TokenizerBackend`], `token_count` must be populated from that
/// backend's count instead of staying `None` forever.
#[cfg(feature = "chunking-tokenizers")]
#[test]
fn test_token_count_populated_from_registered_tokenizer_backend() {
    use crate::plugins::registry::test_support::TokenizerRegistryGuard;
    use crate::plugins::{Plugin, TokenizerBackend, register_tokenizer_backend};
    use std::sync::Arc;

    struct WordCountTokenizer;
    impl Plugin for WordCountTokenizer {
        fn name(&self) -> &str {
            "chunking-core-word-count-tokenizer"
        }
        fn version(&self) -> String {
            "1.0.0".to_string()
        }
        fn initialize(&self) -> crate::Result<()> {
            Ok(())
        }
        fn shutdown(&self) -> crate::Result<()> {
            Ok(())
        }
    }
    impl TokenizerBackend for WordCountTokenizer {
        fn count_tokens(&self, text: &str) -> usize {
            text.split_whitespace().count()
        }
    }

    let _guard = TokenizerRegistryGuard::acquire();
    register_tokenizer_backend(Arc::new(WordCountTokenizer)).unwrap();

    let config = ChunkingConfig {
        max_characters: 100,
        overlap: 0,
        trim: true,
        chunker_type: ChunkerType::Text,
        sizing: crate::core::config::ChunkSizing::Tokenizer {
            model: "chunking-core-word-count-tokenizer".to_string(),
            cache_dir: None,
        },
        ..Default::default()
    };
    let text = "one two three four five";
    let result = chunk_text(text, &config, None).unwrap();

    assert_eq!(result.chunks.len(), 1);
    assert_eq!(
        result.chunks[0].metadata.token_count,
        Some(5),
        "token_count must be populated from the registered tokenizer backend, not left None"
    );
}

/// `token_count` has no meaningful value for character-based sizing (the default);
/// it must stay `None` rather than being populated with a bogus count.
#[test]
fn test_token_count_stays_none_for_character_sizing() {
    let config = ChunkingConfig::default();
    let result = chunk_text("hello world", &config, None).unwrap();
    assert_eq!(result.chunks.len(), 1);
    assert_eq!(result.chunks[0].metadata.token_count, None);
}

/// Regression test for #211: heading-context-aware classification rules (e.g.
/// `is_schedule`, which inspects the *heading text*, not just the chunk body) are
/// dead at `build_chunks` time because `heading_context` is always `None` there —
/// it is only resolved afterward. After the fix, chunks must be reclassified using
/// the now-known `heading_context`.
#[test]
fn test_chunk_reclassified_after_heading_context_resolved() {
    let heading = "# Schedule 1";
    // No schedule/annex/appendix/exhibit keyword in the body itself, and not
    // matching any other higher-priority rule (heading/code/table/formula/
    // definitions/signature/operative-clause/party-list), so this body classifies
    // as `Unknown` on its own — only the heading context can make it `Schedule`.
    let body = "This paragraph states general provisions applicable under the arrangement.";
    let markdown = format!("{heading}\n\n{body}");

    // Small enough that the heading and body land in separate chunks, so the body
    // chunk's own content never starts with '#' (which would otherwise trip the
    // higher-priority `is_heading` rule instead of exercising `is_schedule`).
    let config = ChunkingConfig {
        max_characters: body.len(),
        overlap: 0,
        trim: true,
        chunker_type: ChunkerType::Markdown,
        ..Default::default()
    };
    let result = chunk_text(&markdown, &config, None).unwrap();

    let body_chunk = result
        .chunks
        .iter()
        .find(|c| c.content.trim() == body)
        .unwrap_or_else(|| {
            panic!(
                "expected a chunk containing exactly the body text, got: {:?}",
                result.chunks
            )
        });

    assert!(
        body_chunk.metadata.heading_context.is_some(),
        "body chunk must have heading_context resolved"
    );
    assert_eq!(
        body_chunk.chunk_type,
        crate::types::ChunkType::Schedule,
        "chunk under a 'Schedule 1' heading must reclassify as Schedule once heading_context is known, got {:?}",
        body_chunk.chunk_type
    );
}
