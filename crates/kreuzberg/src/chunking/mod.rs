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
//! let result = chunk_text(&long_text, &config)?;
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
use crate::types::{Chunk, ChunkMetadata};
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

pub fn chunk_text(text: &str, config: &ChunkingConfig) -> Result<ChunkingResult> {
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
            let char_end = char_start + chunk_text.chars().count();
            char_offset = char_end;

            Chunk {
                content: chunk_text.to_string(),
                embedding: None,
                metadata: ChunkMetadata {
                    char_start,
                    char_end,
                    token_count: None,
                    chunk_index: index,
                    total_chunks,
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
    chunk_text(text, &config)
}

pub fn chunk_texts_batch(texts: &[&str], config: &ChunkingConfig) -> Result<Vec<ChunkingResult>> {
    texts.iter().map(|text| chunk_text(text, config)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_empty_text() {
        let config = ChunkingConfig::default();
        let result = chunk_text("", &config).unwrap();
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
        let result = chunk_text(text, &config).unwrap();
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
        let result = chunk_text(text, &config).unwrap();
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
        let result = chunk_text(text, &config).unwrap();
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
        let result = chunk_text(markdown, &config).unwrap();
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
        let result = chunk_text(markdown, &config).unwrap();
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
        let result = chunk_text(markdown, &config).unwrap();
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
        let result = chunk_text(text, &config).unwrap();
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
        let result = chunk_text(text, &config).unwrap();
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
        let result = chunk_text("Some text", &config);
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
        let result = chunk_text(&text, &config).unwrap();
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
        let result = chunk_text(text, &config).unwrap();
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
        let result = chunk_text(markdown, &config).unwrap();
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
        let result = chunk_text(markdown, &config).unwrap();
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
        let result = chunk_text(text, &config).unwrap();
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
        let result = chunk_text(text, &config).unwrap();
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
        let result = chunk_text(text, &config).unwrap();
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
        let result = chunk_text(text, &config).unwrap();
        assert!(result.chunk_count >= 1);
    }
}
