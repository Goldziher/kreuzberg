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

/// Validates that byte offsets in page boundaries fall on valid UTF-8 character boundaries.
///
/// This function ensures that all page boundary positions are at valid UTF-8 character
/// boundaries within the text. This is CRITICAL to prevent text corruption when boundaries
/// are created from language bindings or external sources, particularly with multibyte
/// UTF-8 characters (emoji, CJK characters, combining marks, etc.).
///
/// # Arguments
///
/// * `text` - The text being chunked
/// * `boundaries` - Page boundary markers to validate
///
/// # Returns
///
/// Returns `Ok(())` if all boundaries are at valid UTF-8 character boundaries.
/// Returns `KreuzbergError::Validation` if any boundary is at an invalid position.
///
/// # UTF-8 Boundary Safety
///
/// Rust strings use UTF-8 encoding where characters can be 1-4 bytes. For example:
/// - ASCII letters: 1 byte each
/// - Emoji (🌍): 4 bytes but 1 character
/// - CJK characters (中): 3 bytes but 1 character
///
/// This function checks that all byte_start and byte_end values are at character
/// boundaries using Rust's `is_char_boundary()` method.
fn validate_utf8_boundaries(text: &str, boundaries: &[PageBoundary]) -> Result<()> {
    for (idx, boundary) in boundaries.iter().enumerate() {
        // Check byte_start
        if boundary.byte_start > 0 && boundary.byte_start <= text.len() {
            if !text.is_char_boundary(boundary.byte_start) {
                return Err(KreuzbergError::validation(format!(
                    "Page boundary {} has byte_start={} which is not a valid UTF-8 character boundary (text length={}). This may indicate corrupted multibyte characters (emoji, CJK, etc.)",
                    idx, boundary.byte_start, text.len()
                )));
            }
        } else if boundary.byte_start > text.len() {
            return Err(KreuzbergError::validation(format!(
                "Page boundary {} has byte_start={} which exceeds text length {}",
                idx, boundary.byte_start, text.len()
            )));
        }

        // Check byte_end
        if boundary.byte_end > 0 && boundary.byte_end <= text.len() {
            if !text.is_char_boundary(boundary.byte_end) {
                return Err(KreuzbergError::validation(format!(
                    "Page boundary {} has byte_end={} which is not a valid UTF-8 character boundary (text length={}). This may indicate corrupted multibyte characters (emoji, CJK, etc.)",
                    idx, boundary.byte_end, text.len()
                )));
            }
        } else if boundary.byte_end > text.len() {
            return Err(KreuzbergError::validation(format!(
                "Page boundary {} has byte_end={} which exceeds text length {}",
                idx, boundary.byte_end, text.len()
            )));
        }
    }

    Ok(())
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
/// Validates page boundaries for consistency and correctness.
///
/// # Validation Rules
///
/// 1. Boundaries must be sorted by char_start (monotonically increasing)
/// 2. Boundaries must not overlap (char_end[i] <= char_start[i+1])
/// 3. Each boundary must have char_start < char_end
///
/// # Errors
///
/// Returns `KreuzbergError::Validation` if any boundary is invalid.
fn validate_page_boundaries(boundaries: &[PageBoundary]) -> Result<()> {
    if boundaries.is_empty() {
        return Ok(());
    }

    // Check each boundary individually
    for (idx, boundary) in boundaries.iter().enumerate() {
        // Validate range: byte_start < byte_end
        if boundary.byte_start >= boundary.byte_end {
            return Err(KreuzbergError::validation(format!(
                "Invalid boundary range at index {}: byte_start ({}) must be < byte_end ({})",
                idx, boundary.byte_start, boundary.byte_end
            )));
        }
    }

    // Check ordering and overlap
    for i in 0..boundaries.len() - 1 {
        let current = &boundaries[i];
        let next = &boundaries[i + 1];

        // Check monotonic ordering
        if current.byte_start > next.byte_start {
            return Err(KreuzbergError::validation(format!(
                "Page boundaries not sorted: boundary at index {} (byte_start={}) comes after boundary at index {} (byte_start={})",
                i,
                current.byte_start,
                i + 1,
                next.byte_start
            )));
        }

        // Check for overlap
        if current.byte_end > next.byte_start {
            return Err(KreuzbergError::validation(format!(
                "Overlapping page boundaries: boundary {} ends at {} but boundary {} starts at {}",
                i,
                current.byte_end,
                i + 1,
                next.byte_start
            )));
        }
    }

    Ok(())
}

/// Calculate which pages a byte range spans.
///
/// # Arguments
///
/// * `byte_start` - Starting byte offset of the chunk
/// * `byte_end` - Ending byte offset of the chunk
/// * `boundaries` - Page boundary markers from the document
///
/// # Returns
///
/// A tuple of (first_page, last_page) where page numbers are 1-indexed.
/// Returns (None, None) if boundaries are empty or chunk doesn't overlap any page.
///
/// # Errors
///
/// Returns `KreuzbergError::Validation` if boundaries are invalid.
fn calculate_page_range(
    byte_start: usize,
    byte_end: usize,
    boundaries: &[PageBoundary],
) -> Result<(Option<usize>, Option<usize>)> {
    if boundaries.is_empty() {
        return Ok((None, None));
    }

    // Validate boundaries before processing
    validate_page_boundaries(boundaries)?;

    let mut first_page = None;
    let mut last_page = None;

    for boundary in boundaries {
        // Check if chunk overlaps this page
        if byte_start < boundary.byte_end && byte_end > boundary.byte_start {
            if first_page.is_none() {
                first_page = Some(boundary.page_number);
            }
            last_page = Some(boundary.page_number);
        }
    }

    Ok((first_page, last_page))
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

    // Validate UTF-8 boundaries if provided - critical to prevent text corruption
    if let Some(boundaries) = page_boundaries {
        validate_utf8_boundaries(text, boundaries)?;
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
    let mut byte_offset = 0;

    let mut chunks: Vec<Chunk> = Vec::new();

    for (index, chunk_text) in text_chunks.into_iter().enumerate() {
        let byte_start = byte_offset;
        let chunk_length = chunk_text.len();
        let byte_end = byte_start + chunk_length;

        let overlap_chars = if index < total_chunks - 1 {
            config.overlap.min(chunk_length)
        } else {
            0
        };
        byte_offset = byte_end - overlap_chars;

        // Calculate page range for this chunk
        let (first_page, last_page) = if let Some(boundaries) = page_boundaries {
            calculate_page_range(byte_start, byte_end, boundaries)?
        } else {
            (None, None)
        };

        chunks.push(Chunk {
            content: chunk_text.to_string(),
            embedding: None,
            metadata: ChunkMetadata {
                byte_start,
                byte_end,
                token_count: None,
                chunk_index: index,
                total_chunks,
                first_page,
                last_page,
            },
        });
    }

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
        use crate::types::PageBoundary;

        let config = ChunkingConfig {
            max_characters: 30,
            overlap: 5,
            trim: true,
            chunker_type: ChunkerType::Text,
        };
        let text = "Page one content here. Page two starts here and continues.";

        // Define page boundaries: page 1 is bytes 0-21, page 2 is bytes 22-58
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

        // Check that chunks spanning pages are properly marked
        for chunk in &result.chunks {
            // Each chunk should have at least a first_page when boundaries exist
            assert!(chunk.metadata.first_page.is_some() || chunk.metadata.last_page.is_some());
        }
    }

    #[test]
    fn test_chunk_text_with_invalid_boundary_range() {
        use crate::types::PageBoundary;

        let config = ChunkingConfig {
            max_characters: 30,
            overlap: 5,
            trim: true,
            chunker_type: ChunkerType::Text,
        };
        let text = "Page one content here. Page two content.";

        // Invalid: byte_start >= byte_end
        let boundaries = vec![PageBoundary {
            byte_start: 10,
            byte_end: 5, // Invalid: end < start
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
        use crate::types::PageBoundary;

        let config = ChunkingConfig {
            max_characters: 30,
            overlap: 5,
            trim: true,
            chunker_type: ChunkerType::Text,
        };
        let text = "Page one content here. Page two content.";

        // Invalid: boundaries not sorted by char_start
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
        use crate::types::PageBoundary;

        let config = ChunkingConfig {
            max_characters: 30,
            overlap: 5,
            trim: true,
            chunker_type: ChunkerType::Text,
        };
        let text = "Page one content here. Page two content.";

        // Invalid: overlapping boundaries
        let boundaries = vec![
            PageBoundary {
                byte_start: 0,
                byte_end: 25, // Overlaps with next boundary start
                page_number: 1,
            },
            PageBoundary {
                byte_start: 20, // Starts before previous ends
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
    fn test_calculate_page_range_with_invalid_boundaries() {
        use crate::types::PageBoundary;

        // Invalid: byte_start >= byte_end
        let boundaries = vec![PageBoundary {
            byte_start: 15,
            byte_end: 10,
            page_number: 1,
        }];

        let result = calculate_page_range(0, 20, &boundaries);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Invalid boundary range"));
    }

    #[test]
    fn test_validate_page_boundaries_valid() {
        use crate::types::PageBoundary;

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
                byte_end: 60,
                page_number: 3,
            },
        ];

        // Should not error on valid boundaries
        let result = chunk_text(
            "x".repeat(60).as_str(),
            &ChunkingConfig {
                max_characters: 30,
                overlap: 5,
                trim: false,
                chunker_type: ChunkerType::Text,
            },
            Some(&boundaries),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_page_boundaries_empty() {
        // Empty boundaries should be valid (no boundaries to validate)
        let boundaries: Vec<PageBoundary> = vec![];
        let result = chunk_text(
            "Some test text",
            &ChunkingConfig {
                max_characters: 30,
                overlap: 5,
                trim: true,
                chunker_type: ChunkerType::Text,
            },
            Some(&boundaries),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_page_boundaries_with_gaps() {
        use crate::types::PageBoundary;

        // Valid: boundaries with gaps are allowed
        let boundaries = vec![
            PageBoundary {
                byte_start: 0,
                byte_end: 10,
                page_number: 1,
            },
            PageBoundary {
                byte_start: 15, // Gap from 10 to 15
                byte_end: 25,
                page_number: 2,
            },
        ];

        let text = "0123456789XXXXX0123456789";
        let result = chunk_text(
            text,
            &ChunkingConfig {
                max_characters: 30,
                overlap: 5,
                trim: false,
                chunker_type: ChunkerType::Text,
            },
            Some(&boundaries),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_chunk_with_same_start_and_end() {
        use crate::types::PageBoundary;

        // Invalid: byte_start == byte_end
        let boundaries = vec![PageBoundary {
            byte_start: 10,
            byte_end: 10,
            page_number: 1,
        }];

        let result = chunk_text(
            "test content here",
            &ChunkingConfig {
                max_characters: 30,
                overlap: 5,
                trim: true,
                chunker_type: ChunkerType::Text,
            },
            Some(&boundaries),
        );
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Invalid boundary range"));
    }

    #[test]
    fn test_multiple_overlapping_errors() {
        use crate::types::PageBoundary;

        // Multiple errors: both unsorted AND overlapping
        let text = "This is a longer test content string that spans more bytes";
        let boundaries = vec![
            PageBoundary {
                byte_start: 20,
                byte_end: 40,
                page_number: 2,
            },
            PageBoundary {
                byte_start: 10, // Not sorted
                byte_end: 35,   // Also overlaps with previous
                page_number: 1,
            },
        ];

        let result = chunk_text(
            text,
            &ChunkingConfig {
                max_characters: 30,
                overlap: 5,
                trim: true,
                chunker_type: ChunkerType::Text,
            },
            Some(&boundaries),
        );
        assert!(result.is_err());
        // Should catch unsorted issue first
        assert!(result.unwrap_err().to_string().contains("not sorted"));
    }

    #[test]
    fn test_chunk_with_pages_basic() {
        use crate::types::PageBoundary;

        let config = ChunkingConfig {
            max_characters: 25,
            overlap: 5,
            trim: true,
            chunker_type: ChunkerType::Text,
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

        // First chunk should be on page 1
        if !result.chunks.is_empty() {
            assert!(result.chunks[0].metadata.first_page.is_some());
        }
    }

    #[test]
    fn test_chunk_with_pages_single_page_chunk() {
        use crate::types::PageBoundary;

        let config = ChunkingConfig {
            max_characters: 100,
            overlap: 10,
            trim: true,
            chunker_type: ChunkerType::Text,
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
        use crate::types::PageBoundary;

        let config = ChunkingConfig {
            max_characters: 20,
            overlap: 0,
            trim: false,
            chunker_type: ChunkerType::Text,
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
        assert!(result.chunks.len() >= 1);

        // Verify no chunks overlap page boundaries incorrectly
        for chunk in &result.chunks {
            if let (Some(first), Some(last)) = (chunk.metadata.first_page, chunk.metadata.last_page) {
                assert!(first <= last);
            }
        }
    }

    #[test]
    fn test_calculate_page_range_within_page() {
        let boundaries = vec![
            PageBoundary {
                byte_start: 0,
                byte_end: 100,
                page_number: 1,
            },
            PageBoundary {
                byte_start: 100,
                byte_end: 200,
                page_number: 2,
            },
        ];

        let (first, last) = calculate_page_range(10, 50, &boundaries).unwrap();
        assert_eq!(first, Some(1));
        assert_eq!(last, Some(1));
    }

    #[test]
    fn test_calculate_page_range_spanning_pages() {
        let boundaries = vec![
            PageBoundary {
                byte_start: 0,
                byte_end: 100,
                page_number: 1,
            },
            PageBoundary {
                byte_start: 100,
                byte_end: 200,
                page_number: 2,
            },
        ];

        let (first, last) = calculate_page_range(50, 150, &boundaries).unwrap();
        assert_eq!(first, Some(1));
        assert_eq!(last, Some(2));
    }

    #[test]
    fn test_calculate_page_range_empty_boundaries() {
        let boundaries: Vec<PageBoundary> = vec![];

        let (first, last) = calculate_page_range(0, 50, &boundaries).unwrap();
        assert_eq!(first, None);
        assert_eq!(last, None);
    }

    #[test]
    fn test_calculate_page_range_no_overlap() {
        let boundaries = vec![
            PageBoundary {
                byte_start: 0,
                byte_end: 100,
                page_number: 1,
            },
            PageBoundary {
                byte_start: 100,
                byte_end: 200,
                page_number: 2,
            },
        ];

        // Chunk completely after all boundaries
        let (first, last) = calculate_page_range(200, 250, &boundaries).unwrap();
        assert_eq!(first, None);
        assert_eq!(last, None);
    }

    #[test]
    fn test_calculate_page_range_three_pages() {
        let boundaries = vec![
            PageBoundary {
                byte_start: 0,
                byte_end: 100,
                page_number: 1,
            },
            PageBoundary {
                byte_start: 100,
                byte_end: 200,
                page_number: 2,
            },
            PageBoundary {
                byte_start: 200,
                byte_end: 300,
                page_number: 3,
            },
        ];

        let (first, last) = calculate_page_range(50, 250, &boundaries).unwrap();
        assert_eq!(first, Some(1));
        assert_eq!(last, Some(3));
    }

    #[test]
    fn test_chunk_metadata_page_range_accuracy() {
        use crate::types::PageBoundary;

        let config = ChunkingConfig {
            max_characters: 30,
            overlap: 5,
            trim: true,
            chunker_type: ChunkerType::Text,
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
                byte_end: 31, // Text is 31 bytes, not 32
                page_number: 2,
            },
        ];

        let result = chunk_text(text, &config, Some(&boundaries)).unwrap();

        for chunk in &result.chunks {
            // Verify byte offsets match content length
            assert_eq!(chunk.metadata.byte_end - chunk.metadata.byte_start, chunk.content.len());
        }
    }

    #[test]
    fn test_chunk_page_range_boundary_edge_cases() {
        use crate::types::PageBoundary;

        let config = ChunkingConfig {
            max_characters: 10,
            overlap: 2,
            trim: false,
            chunker_type: ChunkerType::Text,
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

        // Check chunks at boundary edges
        for chunk in &result.chunks {
            let on_page1 = chunk.metadata.byte_start < 10;
            let on_page2 = chunk.metadata.byte_end > 10;

            if on_page1 && on_page2 {
                // Spanning boundary
                assert_eq!(chunk.metadata.first_page, Some(1));
                assert_eq!(chunk.metadata.last_page, Some(2));
            } else if on_page1 {
                assert_eq!(chunk.metadata.first_page, Some(1));
            } else if on_page2 {
                assert_eq!(chunk.metadata.first_page, Some(2));
            }
        }
    }

    // ========== UTF-8 Boundary Validation Tests ==========

    #[test]
    fn test_validate_utf8_boundaries_valid_ascii() {
        use crate::types::PageBoundary;

        let text = "This is ASCII text.";
        let boundaries = vec![
            PageBoundary {
                byte_start: 0,
                byte_end: 10,
                page_number: 1,
            },
            PageBoundary {
                byte_start: 10,
                byte_end: 19,
                page_number: 2,
            },
        ];

        // Should not error with valid ASCII boundaries
        let result = chunk_text(text, &ChunkingConfig::default(), Some(&boundaries));
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_utf8_boundaries_valid_emoji() {
        use crate::types::PageBoundary;

        // Emoji 👋 is 4 bytes at position 6-9, 🌍 is 4 bytes at position 17-20
        // Text: "Hello 👋 World 🌍 End" = 25 bytes total
        let text = "Hello 👋 World 🌍 End";
        let config = ChunkingConfig::default();

        // Valid boundaries on character boundaries
        let boundaries = vec![
            PageBoundary {
                byte_start: 0,
                byte_end: 11, // "Hello 👋 " (H-o=5, space=1, emoji=4, space=1)
                page_number: 1,
            },
            PageBoundary {
                byte_start: 11,
                byte_end: 25, // "World 🌍 End" (rest of text)
                page_number: 2,
            },
        ];

        let result = chunk_text(text, &config, Some(&boundaries));
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_utf8_boundaries_valid_cjk() {
        use crate::types::PageBoundary;

        // CJK characters are 3 bytes each
        // "你好世界 こんにちは 안녕하세요" = 44 bytes total
        let text = "你好世界 こんにちは 안녕하세요";
        let config = ChunkingConfig::default();

        let boundaries = vec![
            PageBoundary {
                byte_start: 0,
                byte_end: 13, // "你好世界 " = 4*3 bytes + 1 space
                page_number: 1,
            },
            PageBoundary {
                byte_start: 13,
                byte_end: 44, // Rest of text
                page_number: 2,
            },
        ];

        let result = chunk_text(text, &config, Some(&boundaries));
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_utf8_boundaries_invalid_mid_emoji() {
        use crate::types::PageBoundary;

        let text = "Hello 👋 World";
        // Emoji 👋 starts at byte 6 and ends at byte 10
        // Trying to set boundary at byte 7, 8, or 9 is invalid
        let boundaries = vec![
            PageBoundary {
                byte_start: 0,
                byte_end: 7, // Mid-emoji (between byte 6 and 10)
                page_number: 1,
            },
        ];

        let config = ChunkingConfig::default();
        let result = chunk_text(text, &config, Some(&boundaries));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("UTF-8 character boundary"));
        assert!(err.to_string().contains("byte_end=7"));
    }

    #[test]
    fn test_validate_utf8_boundaries_invalid_mid_multibyte_cjk() {
        use crate::types::PageBoundary;

        // Chinese character 中 is E4 B8 AD (3 bytes)
        let text = "中文文本";
        // Character 中 is at bytes 0-2, 文 is at 3-5, etc.
        let boundaries = vec![
            PageBoundary {
                byte_start: 0,
                byte_end: 1, // Mid-character (in the middle of 中)
                page_number: 1,
            },
        ];

        let config = ChunkingConfig::default();
        let result = chunk_text(text, &config, Some(&boundaries));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("UTF-8 character boundary"));
    }

    #[test]
    fn test_validate_utf8_boundaries_byte_start_exceeds_length() {
        use crate::types::PageBoundary;

        let text = "Short";
        let boundaries = vec![
            PageBoundary {
                byte_start: 0,
                byte_end: 3,
                page_number: 1,
            },
            PageBoundary {
                byte_start: 10, // Exceeds text byte length
                byte_end: 15,
                page_number: 2,
            },
        ];

        let config = ChunkingConfig::default();
        let result = chunk_text(text, &config, Some(&boundaries));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("exceeds text length"));
    }

    #[test]
    fn test_validate_utf8_boundaries_byte_end_exceeds_length() {
        use crate::types::PageBoundary;

        let text = "Short";
        let boundaries = vec![
            PageBoundary {
                byte_start: 0,
                byte_end: 100, // Way exceeds text length
                page_number: 1,
            },
        ];

        let config = ChunkingConfig::default();
        let result = chunk_text(text, &config, Some(&boundaries));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("exceeds text length"));
    }

    #[test]
    fn test_validate_utf8_boundaries_empty_boundaries() {
        use crate::types::PageBoundary;

        let text = "Some text";
        let boundaries: Vec<PageBoundary> = vec![];

        let config = ChunkingConfig::default();
        let result = chunk_text(text, &config, Some(&boundaries));
        // Empty boundaries should be valid
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_utf8_boundaries_at_text_boundaries() {
        use crate::types::PageBoundary;

        let text = "Exact boundary test";
        let text_len = text.len();
        let boundaries = vec![
            PageBoundary {
                byte_start: 0,
                byte_end: text_len, // Exactly at end
                page_number: 1,
            },
        ];

        let config = ChunkingConfig::default();
        let result = chunk_text(text, &config, Some(&boundaries));
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_utf8_boundaries_mixed_languages() {
        use crate::types::PageBoundary;

        let text = "English text mixed with 中文 and français";
        let config = ChunkingConfig::default();

        // Boundaries at valid character positions
        let boundaries = vec![
            PageBoundary {
                byte_start: 0,
                byte_end: 24, // "English text mixed with "
                page_number: 1,
            },
            PageBoundary {
                byte_start: 24,
                byte_end: text.len(),
                page_number: 2,
            },
        ];

        let result = chunk_text(text, &config, Some(&boundaries));
        assert!(result.is_ok());
    }

    #[test]
    fn test_chunk_text_rejects_invalid_utf8_boundaries() {
        use crate::types::PageBoundary;

        let text = "🌍🌎🌏 Three emoji planets";
        let config = ChunkingConfig::default();

        // Set boundary way out of bounds
        let boundaries = vec![
            PageBoundary {
                byte_start: 0,
                byte_end: 1000, // Way out of bounds
                page_number: 1,
            },
        ];

        let result = chunk_text(text, &config, Some(&boundaries));
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_utf8_boundaries_combining_diacriticals() {
        use crate::types::PageBoundary;

        // Text with combining diacritical marks: "café" with combining acute
        let text = "café"; // Normally precomposed, but test the concept
        let config = ChunkingConfig::default();

        let boundaries = vec![
            PageBoundary {
                byte_start: 0,
                byte_end: 2,
                page_number: 1,
            },
            PageBoundary {
                byte_start: 2,
                byte_end: text.len(),
                page_number: 2,
            },
        ];

        let result = chunk_text(text, &config, Some(&boundaries));
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_utf8_boundaries_error_messages_are_clear() {
        use crate::types::PageBoundary;

        let text = "Test 👋 text";
        let config = ChunkingConfig::default();

        let boundaries = vec![
            PageBoundary {
                byte_start: 0,
                byte_end: 6, // Mid-emoji
                page_number: 1,
            },
        ];

        let result = chunk_text(text, &config, Some(&boundaries));
        assert!(result.is_err());
        let err = result.unwrap_err();
        let err_msg = err.to_string();
        // Check that error message is descriptive
        assert!(err_msg.contains("UTF-8"));
        assert!(err_msg.contains("boundary"));
        assert!(err_msg.contains("6")); // The invalid offset
    }

    #[test]
    fn test_validate_utf8_boundaries_multiple_valid_boundaries() {
        use crate::types::PageBoundary;

        let text = "First👋Second🌍Third";
        let config = ChunkingConfig::default();

        // Multiple valid boundaries
        let boundaries = vec![
            PageBoundary {
                byte_start: 0,
                byte_end: 5, // "First"
                page_number: 1,
            },
            PageBoundary {
                byte_start: 5,
                byte_end: 9, // "👋" (4 bytes)
                page_number: 2,
            },
            PageBoundary {
                byte_start: 9,
                byte_end: 15, // "Second"
                page_number: 3,
            },
            PageBoundary {
                byte_start: 15,
                byte_end: 19, // "🌍" (4 bytes)
                page_number: 4,
            },
            PageBoundary {
                byte_start: 19,
                byte_end: text.len(), // "Third"
                page_number: 5,
            },
        ];

        let result = chunk_text(text, &config, Some(&boundaries));
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_utf8_boundaries_zero_start_and_end() {
        use crate::types::PageBoundary;

        let text = "Text";
        let config = ChunkingConfig::default();

        // Valid boundaries with 0 values
        let boundaries = vec![
            PageBoundary {
                byte_start: 0,
                byte_end: 0,
                page_number: 1,
            },
        ];

        // Zero-length boundary should fail validation from page_boundaries
        // but UTF-8 validation should pass (0 is always valid)
        let result = chunk_text(text, &config, Some(&boundaries));
        // This should fail due to page boundary validation, not UTF-8
        assert!(result.is_err());
    }
}
