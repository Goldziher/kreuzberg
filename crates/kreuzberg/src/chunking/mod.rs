//! Text chunking utilities.
//!
//! This module provides text chunking functionality using the `text-splitter` library.
//! It splits long text into smaller chunks while preserving semantic boundaries.
//!
//! # Features
//!
//! - **Smart splitting**: Respects word and sentence boundaries
//! - **Markdown-aware**: Preserves Markdown structure (headings, code blocks, lists)
//! - **Configurable overlap**: Overlap chunks to maintain context
//! - **Unicode support**: Handles CJK characters and emojis correctly
//! - **Batch processing**: Process multiple texts efficiently
//!
//! # Chunker Types
//!
//! - **Text**: Generic text splitter, splits on whitespace and punctuation
//! - **Markdown**: Markdown-aware splitter, preserves formatting and structure
//!
//! # Example
//!
//! ```rust
//! use kreuzberg::chunking::{chunk_text, ChunkingConfig, ChunkerType};
//!
//! # fn example() -> kreuzberg::Result<()> {
//! let config = ChunkingConfig {
//!     max_characters: 500,
//!     overlap: 50,
//!     trim: true,
//!     chunker_type: ChunkerType::Text,
//! };
//!
//! let long_text = "This is a very long document...".repeat(100);
//! let result = chunk_text(&long_text, &config, None)?;
//!
//! println!("Split into {} chunks", result.chunk_count);
//! for (i, chunk) in result.chunks.iter().enumerate() {
//!     println!("Chunk {}: {} chars", i + 1, chunk.content.len());
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Use Cases
//!
//! - Splitting documents for LLM context windows
//! - Creating overlapping chunks for semantic search
//! - Processing large documents in batches
//! - Maintaining context across chunk boundaries
use crate::error::{KreuzbergError, Result};
use crate::types::{Chunk, ChunkMetadata, PageBoundary};
use serde::{Deserialize, Serialize};
use text_splitter::{Characters, ChunkCapacity, ChunkConfig, MarkdownSplitter, TextSplitter};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChunkerType {
    Text,
    Markdown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkingResult {
    pub chunks: Vec<Chunk>,
    pub chunk_count: usize,
}

pub struct ChunkingConfig {
    pub max_characters: usize,
    pub overlap: usize,
    pub trim: bool,
    pub chunker_type: ChunkerType,
}

impl Default for ChunkingConfig {
    fn default() -> Self {
        Self {
            max_characters: 2000,
            overlap: 100,
            trim: true,
            chunker_type: ChunkerType::Text,
        }
    }
}

fn build_chunk_config(max_characters: usize, overlap: usize, trim: bool) -> Result<ChunkConfig<Characters>> {
    ChunkConfig::new(ChunkCapacity::new(max_characters))
        .with_overlap(overlap)
        .map(|config| config.with_trim(trim))
        .map_err(|e| KreuzbergError::validation(format!("Invalid chunking configuration: {}", e)))
}

/// Calculate which pages a character range spans.
///
/// # Arguments
///
/// * `char_start` - Starting character offset of the chunk
/// * `char_end` - Ending character offset of the chunk
/// * `boundaries` - Page boundary markers from the document
///
/// # Returns
///
/// A tuple of (first_page, last_page) where page numbers are 1-indexed.
/// Returns (None, None) if boundaries are empty or chunk doesn't overlap any page.
fn calculate_page_range(
    char_start: usize,
    char_end: usize,
    boundaries: &[PageBoundary],
) -> (Option<usize>, Option<usize>) {
    if boundaries.is_empty() {
        return (None, None);
    }

    let mut first_page = None;
    let mut last_page = None;

    for boundary in boundaries {
        // Check if chunk overlaps this page
        if char_start < boundary.char_end && char_end > boundary.char_start {
            if first_page.is_none() {
                first_page = Some(boundary.page_number);
            }
            last_page = Some(boundary.page_number);
        }
    }

    (first_page, last_page)
}

/// Split text into chunks with optional page boundary tracking.
///
/// # Arguments
///
/// * `text` - The text to split into chunks
/// * `config` - Chunking configuration (max size, overlap, type)
/// * `page_boundaries` - Optional page boundary markers for mapping chunks to pages
///
/// # Returns
///
/// A ChunkingResult containing all chunks and their metadata.
///
/// # Examples
///
/// ```rust
/// use kreuzberg::chunking::{chunk_text, ChunkingConfig, ChunkerType};
///
/// # fn example() -> kreuzberg::Result<()> {
/// let config = ChunkingConfig {
///     max_characters: 500,
///     overlap: 50,
///     trim: true,
///     chunker_type: ChunkerType::Text,
/// };
/// let result = chunk_text("Long text...", &config, None)?;
/// assert!(!result.chunks.is_empty());
/// # Ok(())
/// # }
/// ```
pub fn chunk_text(
    text: &str,
    config: &ChunkingConfig,
    page_boundaries: Option<&[PageBoundary]>,
) -> Result<ChunkingResult> {
    if text.is_empty() {
        return Ok(ChunkingResult {
            chunks: vec![],
            chunk_count: 0,
        });
    }

    let chunk_config = build_chunk_config(config.max_characters, config.overlap, config.trim)?;

    let text_chunks: Vec<&str> = match config.chunker_type {
        ChunkerType::Text => {
            let splitter = TextSplitter::new(chunk_config);
            splitter.chunks(text).collect()
        }
        ChunkerType::Markdown => {
            let splitter = MarkdownSplitter::new(chunk_config);
            splitter.chunks(text).collect()
        }
    };

    let total_chunks = text_chunks.len();
    let mut char_offset = 0;

    let chunks: Vec<Chunk> = text_chunks
        .into_iter()
        .enumerate()
        .map(|(index, chunk_text)| {
            let char_start = char_offset;
            let chunk_length = chunk_text.chars().count();
            let char_end = char_start + chunk_length;

            let overlap_chars = if index < total_chunks - 1 {
                config.overlap.min(chunk_length)
            } else {
                0
            };
            char_offset = char_end - overlap_chars;

            // Calculate page range for this chunk
            let (first_page, last_page) = if let Some(boundaries) = page_boundaries {
                calculate_page_range(char_start, char_end, boundaries)
            } else {
                (None, None)
            };

            Chunk {
                content: chunk_text.to_string(),
                embedding: None,
                metadata: ChunkMetadata {
                    char_start,
                    char_end,
                    token_count: None,
                    chunk_index: index,
                    total_chunks,
                    first_page,
                    last_page,
                },
            }
        })
        .collect();

    let chunk_count = chunks.len();

    Ok(ChunkingResult { chunks, chunk_count })
}

pub fn chunk_text_with_type(
    text: &str,
    max_characters: usize,
    overlap: usize,
    trim: bool,
    chunker_type: ChunkerType,
) -> Result<ChunkingResult> {
    let config = ChunkingConfig {
        max_characters,
        overlap,
        trim,
        chunker_type,
    };
    chunk_text(text, &config, None)
}

pub fn chunk_texts_batch(texts: &[&str], config: &ChunkingConfig) -> Result<Vec<ChunkingResult>> {
    texts.iter().map(|text| chunk_text(text, config, None)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn test_chunk_markdown_preserves_structure() {
        let config = ChunkingConfig {
            max_characters: 50,
            overlap: 10,
            trim: true,
            chunker_type: ChunkerType::Markdown,
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
        };
        let result = chunk_text("Some text", &config, None);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, KreuzbergError::Validation { .. }));
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
        let texts: Vec<&str> = vec![];
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
        };
        let texts = vec!["First text", "Second text", "Third text"];
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
        };
        let texts = vec![
            "Short",
            "This is a longer text that should be split into multiple chunks",
            "",
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
        };
        let texts = vec!["Text one", "Text two"];
        let result = chunk_texts_batch(&texts, &config);
        assert!(result.is_err());
    }

    #[test]
    fn test_chunking_config_default() {
        let config = ChunkingConfig::default();
        assert_eq!(config.max_characters, 2000);
        assert_eq!(config.overlap, 100);
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
        };
        let text = "日本語のテキストです。これは長い文章で、複数のチャンクに分割されるべきです。";
        let result = chunk_text(text, &config, None).unwrap();
        assert!(result.chunk_count >= 1);
    }

    #[test]
    fn test_chunk_mixed_languages() {
        let config = ChunkingConfig {
            max_characters: 40,
            overlap: 5,
            trim: true,
            chunker_type: ChunkerType::Text,
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
        };
        let text = "AAAAA BBBBB CCCCC DDDDD EEEEE FFFFF";
        let result = chunk_text(text, &config, None).unwrap();

        assert!(result.chunks.len() >= 2, "Expected at least 2 chunks");

        for i in 0..result.chunks.len() {
            let chunk = &result.chunks[i];
            let metadata = &chunk.metadata;

            assert_eq!(
                metadata.char_end - metadata.char_start,
                chunk.content.chars().count(),
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
                next_chunk.metadata.char_start < current_chunk.metadata.char_end,
                "Chunk {} and {} don't overlap: next starts at {} but current ends at {}",
                i,
                i + 1,
                next_chunk.metadata.char_start,
                current_chunk.metadata.char_end
            );

            let overlap_size = current_chunk.metadata.char_end - next_chunk.metadata.char_start;
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
        };
        let text = "AAAAA BBBBB CCCCC DDDDD EEEEE FFFFF";
        let result = chunk_text(text, &config, None).unwrap();

        for i in 0..result.chunks.len() - 1 {
            let current_chunk = &result.chunks[i];
            let next_chunk = &result.chunks[i + 1];

            assert!(
                next_chunk.metadata.char_start >= current_chunk.metadata.char_end,
                "Chunk {} and {} overlap when they shouldn't: next starts at {} but current ends at {}",
                i,
                i + 1,
                next_chunk.metadata.char_start,
                current_chunk.metadata.char_end
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
        };
        let text = "0123456789 ABCDEFGHIJ KLMNOPQRST UVWXYZ";
        let result = chunk_text(text, &config, None).unwrap();

        assert!(result.chunks.len() >= 2, "Expected multiple chunks");

        assert_eq!(
            result.chunks[0].metadata.char_start, 0,
            "First chunk should start at position 0"
        );

        for i in 0..result.chunks.len() - 1 {
            let current_chunk = &result.chunks[i];
            let next_chunk = &result.chunks[i + 1];

            assert!(
                next_chunk.metadata.char_start <= current_chunk.metadata.char_end,
                "Gap detected between chunk {} (ends at {}) and chunk {} (starts at {})",
                i,
                current_chunk.metadata.char_end,
                i + 1,
                next_chunk.metadata.char_start
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
            };
            let text = "Word ".repeat(30);
            let result = chunk_text(&text, &config, None).unwrap();

            for chunk in &result.chunks {
                assert!(
                    chunk.metadata.char_end > chunk.metadata.char_start,
                    "Invalid offset range for overlap {}: start={}, end={}",
                    overlap,
                    chunk.metadata.char_start,
                    chunk.metadata.char_end
                );
            }

            for chunk in &result.chunks {
                assert!(
                    chunk.metadata.char_start < text.chars().count(),
                    "char_start with overlap {} is out of bounds: {}",
                    overlap,
                    chunk.metadata.char_start
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
        };
        let text = "AAAAA BBBBB CCCCC DDDDD EEEEE";
        let result = chunk_text(text, &config, None).unwrap();

        assert!(result.chunks.len() >= 2, "Need multiple chunks for this test");

        let last_chunk = result.chunks.last().unwrap();
        let second_to_last = &result.chunks[result.chunks.len() - 2];

        assert!(
            last_chunk.metadata.char_start < second_to_last.metadata.char_end,
            "Last chunk should overlap with previous chunk"
        );

        let expected_end = text.chars().count();
        let last_chunk_covers_end =
            last_chunk.content.trim_end() == text.trim_end() || last_chunk.metadata.char_end >= expected_end - 5;
        assert!(last_chunk_covers_end, "Last chunk should cover the end of the text");
    }

    #[test]
    fn test_chunk_with_page_boundaries() {
        use crate::types::PageBoundary;

        let config = ChunkingConfig {
            max_characters: 30,
            overlap: 5,
            trim: true,
            chunker_type: ChunkerType::Text,
        };
        let text = "Page one content here. Page two starts here and continues.";

        // Define page boundaries: page 1 is chars 0-21, page 2 is chars 22-58
        let boundaries = vec![
            PageBoundary {
                char_start: 0,
                char_end: 21,
                page_number: 1,
            },
            PageBoundary {
                char_start: 22,
                char_end: 58,
                page_number: 2,
            },
        ];

        let result = chunk_text(text, &config, Some(&boundaries)).unwrap();
        assert!(result.chunks.len() >= 2);

        // First chunk should be on page 1
        assert_eq!(result.chunks[0].metadata.first_page, Some(1));

        // Last chunk should be on page 2
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
        };
        let text = "This is some test content that should be split into multiple chunks.";

        let result = chunk_text(text, &config, None).unwrap();
        assert!(result.chunks.len() >= 2);

        // Without page boundaries, all chunks should have None for page info
        for chunk in &result.chunks {
            assert_eq!(chunk.metadata.first_page, None);
            assert_eq!(chunk.metadata.last_page, None);
        }
    }

    #[test]
    fn test_chunk_empty_boundaries() {
        let config = ChunkingConfig {
            max_characters: 30,
            overlap: 5,
            trim: true,
            chunker_type: ChunkerType::Text,
        };
        let text = "Some text content here.";
        let boundaries: Vec<PageBoundary> = vec![];

        let result = chunk_text(text, &config, Some(&boundaries)).unwrap();
        assert_eq!(result.chunks.len(), 1);

        // With empty boundaries, pages should be None
        assert_eq!(result.chunks[0].metadata.first_page, None);
        assert_eq!(result.chunks[0].metadata.last_page, None);
    }

    #[test]
    fn test_chunk_spanning_multiple_pages() {
        use crate::types::PageBoundary;

        let config = ChunkingConfig {
            max_characters: 50,
            overlap: 5,
            trim: false,
            chunker_type: ChunkerType::Text,
        };
        let text = "0123456789 AAAAAAAAAA 1111111111 BBBBBBBBBB 2222222222";

        let boundaries = vec![
            PageBoundary {
                char_start: 0,
                char_end: 20,
                page_number: 1,
            },
            PageBoundary {
                char_start: 20,
                char_end: 40,
                page_number: 2,
            },
            PageBoundary {
                char_start: 40,
                char_end: 56,
                page_number: 3,
            },
        ];

        let result = chunk_text(text, &config, Some(&boundaries)).unwrap();
        assert!(result.chunks.len() >= 2);

        // Check that chunks spanning pages are properly marked
        for chunk in &result.chunks {
            // Each chunk should have at least a first_page when boundaries exist
            assert!(chunk.metadata.first_page.is_some() || chunk.metadata.last_page.is_some());
        }
    }
}
