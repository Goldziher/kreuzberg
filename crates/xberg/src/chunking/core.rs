//! Core text chunking logic and public API.
//!
//! This module implements the main chunking algorithms and provides the primary
//! public API functions for splitting text into chunks.

use std::fmt::Write;

#[cfg(feature = "chunking-tokenizers")]
use crate::chunking::text_splitter::ChunkCapacity;
use crate::chunking::text_splitter::{ChunkConfig, ChunkSizer, MarkdownSplitter, TextSplitter};
use crate::error::Result;
use crate::types::{HeadingContext, PageBoundary};

use super::boundaries::PageIndex;
#[cfg(feature = "chunking-tokenizers")]
use super::builder::TokenizerBackendSizer;
use super::builder::{build_chunk_config, build_chunks, resolve_token_counter};
use super::classifier::classify_chunk;
use super::config::{ChunkerType, ChunkingConfig, ChunkingResult, TableChunkingMode};
use super::headings::{build_heading_map, resolve_heading_context};
use super::validation::validate_utf8_boundaries;

/// Split text into chunks with optional page boundary tracking.
///
/// This is the primary API function for chunking text. It supports both plain text
/// and Markdown with configurable chunk size, overlap, and page boundary mapping.
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
/// use xberg::chunking::{chunk_text, ChunkingConfig, ChunkerType};
///
/// # fn example() -> xberg::Result<()> {
/// let config = ChunkingConfig {
///     max_characters: 500,
///     overlap: 50,
///     trim: true,
///     chunker_type: ChunkerType::Text,
///     ..Default::default()
/// };
/// let result = chunk_text("Long text...", &config, None)?;
/// assert!(!result.chunks.is_empty());
/// # Ok(())
/// # }
/// ```
#[cfg_attr(alef, alef(skip))]
pub fn chunk_text(
    text: &str,
    config: &ChunkingConfig,
    page_boundaries: Option<&[PageBoundary]>,
) -> Result<ChunkingResult> {
    let resolved_config = config.resolve_preset();
    chunk_text_with_heading_source(text, &resolved_config, page_boundaries, None)
}

/// Chunk text with an optional separate markdown source for heading context resolution.
///
/// When `heading_source` is provided, it is used instead of `text` for building the
/// heading map. This is needed when `text` is plain text (no markdown headings) but
/// the original document had headings that were stripped during rendering.
pub(crate) fn chunk_text_with_heading_source(
    text: &str,
    config: &ChunkingConfig,
    page_boundaries: Option<&[PageBoundary]>,
    heading_source: Option<&str>,
) -> Result<ChunkingResult> {
    if text.is_empty() {
        return Ok(ChunkingResult {
            chunks: vec![],
            chunk_count: 0,
        });
    }

    if let Some(boundaries) = page_boundaries {
        validate_utf8_boundaries(text, boundaries)?;
    }

    if config.chunker_type == ChunkerType::Yaml {
        return super::yaml_section::chunk_yaml_by_sections(text, config, page_boundaries);
    }

    if config.chunker_type == ChunkerType::Semantic {
        return super::semantic::chunk_semantic(text, config, page_boundaries);
    }

    let text_chunks = split_text_by_sizing(text, config)?;

    let mut chunks = build_chunks(text, text_chunks, page_boundaries)?;

    if config.chunker_type == ChunkerType::Markdown {
        let heading_map = build_heading_map(heading_source.unwrap_or(text));
        if !heading_map.is_empty() {
            let page_index = page_boundaries.map(PageIndex::new).transpose()?;
            for chunk in &mut chunks {
                chunk.metadata.heading_context =
                    resolve_heading_context(chunk.metadata.byte_start, &heading_map, page_index.as_ref());
                // Reclassify now that heading context is known: heading-context-aware
                // rules (e.g. `is_schedule`) are dead at construction time, where
                // `build_chunks` always classifies with `heading_context = None` (#211).
                chunk.chunk_type = classify_chunk(&chunk.content, chunk.metadata.heading_context.as_ref());
            }

            // Chunk content must remain the exact source span. Retrieval consumers
            // choose whether to add heading context with `render_heading_breadcrumb`;
            // mutating content here would invalidate byte offsets and force one
            // representation on dense, lexical, and sparse indexes. ~keep
        }
    }

    if config.chunker_type == ChunkerType::Markdown && config.table_chunking == TableChunkingMode::RepeatHeader {
        inject_table_headers(&mut chunks);
    }

    // Populate `token_count` from the configured tokenizer sizer, if any (#255).
    // Computed last so it reflects each chunk's final `content` (after any
    // table-header repetition above, the only remaining step that legitimately
    // changes `content` past `build_chunks`). `token_count` must describe the
    // content it is stored next to, so it is never counted against a
    // heading-breadcrumb prefix that no longer exists in `content` (#1393).
    if let Some(counter) = resolve_token_counter(&config.sizing) {
        for chunk in &mut chunks {
            chunk.metadata.token_count = Some(counter(&chunk.content));
        }
    }

    let chunk_count = chunks.len();

    Ok(ChunkingResult { chunks, chunk_count })
}

fn split_text_by_sizing<'a>(text: &'a str, config: &ChunkingConfig) -> Result<Vec<&'a str>> {
    Ok(match &config.sizing {
        #[cfg(feature = "chunking-tokenizers")]
        crate::core::config::ChunkSizing::Tokenizer { model, .. } => {
            let registered = crate::plugins::registry::get_tokenizer_backend_registry()
                .read()
                .lookup(model);
            if let Some(backend) = registered {
                let chunk_config = ChunkConfig::new(ChunkCapacity::new(config.max_characters))
                    .with_sizer(TokenizerBackendSizer(backend))
                    .with_overlap(config.overlap)
                    .map(|c| c.with_trim(config.trim))
                    .map_err(|e| crate::XbergError::validation(format!("Invalid chunking configuration: {}", e)))?;
                split_with_config(text, &config.chunker_type, chunk_config)
            } else {
                let tokenizer = super::tokenizer_cache::get_or_init_tokenizer(model).map_err(|e| match e {
                    crate::XbergError::Validation { message, source } => crate::XbergError::Validation {
                        message: format!(
                            "{message}. No tokenizer backend is registered under '{model}' either; \
                             use a HuggingFace model id, or register your own tokenizer with \
                             register_tokenizer_backend."
                        ),
                        source,
                    },
                    other => other,
                })?;
                let chunk_config = ChunkConfig::new(ChunkCapacity::new(config.max_characters))
                    .with_sizer((*tokenizer).clone())
                    .with_overlap(config.overlap)
                    .map(|c| c.with_trim(config.trim))
                    .map_err(|e| crate::XbergError::validation(format!("Invalid chunking configuration: {}", e)))?;
                split_with_config(text, &config.chunker_type, chunk_config)
            }
        }
        _ => {
            let chunk_config = build_chunk_config(config.max_characters, config.overlap, config.trim)?;
            split_with_config(text, &config.chunker_type, chunk_config)
        }
    })
}

/// Render `content` with its Markdown heading breadcrumb prepended.
///
/// This is the explicit, consumer-applied rendering step described by the
/// revised #1393 design: [`chunk_text`] never mutates `chunk.content` with a
/// heading breadcrumb — `content` always stays exactly the source span
/// (`chunk.content == &source[byte_start..byte_end]`).
/// A caller that wants the breadcrumb inline — e.g. for the dense/embedding
/// retrieval arm, which benefits from a self-contained passage that carries
/// its own structural context — calls this function directly at index time
/// with the chunk's (clean) `content` and its
/// [`heading_context`](crate::types::ChunkMetadata::heading_context), and
/// keeps `content` itself untouched for BM25/SPLADE, which do not want the
/// breadcrumb at all. See the [`rag`](crate::chunking::rag) module docs for
/// the full retrieval-mode guidance.
///
/// Formats `context`'s heading hierarchy as a single breadcrumb line — each
/// heading rendered as ATX hashes followed by its text, joined by `" > "`
/// (e.g. `"# Guide > ## Setup"`) — then a blank line, then `content` with a
/// leading occurrence of the breadcrumb's deepest (last) heading stripped so
/// the heading is not duplicated when `content` itself already starts with
/// that heading line, as Markdown-chunked content commonly does (the
/// Markdown splitter keeps the heading that opens a section as the first
/// line of that section's first chunk).
///
/// # Examples
///
/// ```rust
/// use xberg::chunking::render_heading_breadcrumb;
/// use xberg::types::{HeadingContext, HeadingLevel};
///
/// let context = HeadingContext {
///     headings: vec![
///         HeadingLevel { level: 1, text: "Guide".to_string() },
///         HeadingLevel { level: 2, text: "Setup".to_string() },
///     ],
/// };
/// let rendered = render_heading_breadcrumb("## Setup\n\nInstall the dependencies.", &context);
/// assert_eq!(rendered, "# Guide > ## Setup\n\nInstall the dependencies.");
/// ```
#[cfg_attr(alef, alef(skip))]
pub fn render_heading_breadcrumb(content: &str, context: &HeadingContext) -> String {
    let mut new_content = String::with_capacity(content.len() + 64);
    for (i, h) in context.headings.iter().enumerate() {
        if i > 0 {
            new_content.push_str(" > ");
        }
        for _ in 0..h.level {
            new_content.push('#');
        }
        let _ = write!(new_content, " {}", h.text);
    }
    new_content.push_str("\n\n");

    let body = match context.headings.last() {
        Some(h) => strip_leading_heading(content, h.level, &h.text),
        None => content,
    };
    new_content.push_str(body);
    new_content
}

/// If `text` starts with a markdown ATX heading matching `level` and `title`,
/// return the remainder after that heading line with leading newlines trimmed.
/// Otherwise return the input unchanged.
///
/// Handles optional closing ATX hashes (e.g. `## Heading ##`).
fn strip_leading_heading<'a>(text: &'a str, level: u8, title: &str) -> &'a str {
    debug_assert!(level > 0, "heading level must be 1..=6");
    let n = level as usize;
    let bytes = text.as_bytes();
    if bytes.len() <= n || bytes[..n].iter().any(|&b| b != b'#') || bytes[n] != b' ' {
        return text;
    }
    let after_prefix = &text[n + 1..];
    if !after_prefix.starts_with(title) {
        return text;
    }
    let rest = &after_prefix[title.len()..];
    let line_end = rest.find('\n').unwrap_or(rest.len());
    rest[line_end..].trim_start_matches('\n')
}

/// Split text using the appropriate splitter type with a generic sizer.
fn split_with_config<'a, S: ChunkSizer>(
    text: &'a str,
    chunker_type: &ChunkerType,
    config: ChunkConfig<S>,
) -> Vec<&'a str> {
    match chunker_type {
        ChunkerType::Text | ChunkerType::Yaml | ChunkerType::Semantic => {
            TextSplitter::new(config).chunks(text).collect()
        }
        ChunkerType::Markdown => MarkdownSplitter::new(config).chunks(text).collect(),
    }
}

/// Prepend the table header to every chunk that is a continuation of a split table.
///
/// A "table header" is two consecutive lines where the first starts with `|` and
/// the second is a separator (`|---…` or `|:--…`). A "continuation chunk" is one
/// that starts with a `|`-prefixed line but does NOT already have a separator line
/// within its first two lines — i.e. it is mid-table rows without a header.
///
/// Only called when `table_chunking == RepeatHeader` and `chunker_type == Markdown`.
fn inject_table_headers(chunks: &mut [crate::types::Chunk]) {
    fn extract_table_header(content: &str) -> Option<String> {
        let line_ending = if content.contains("\r\n") { "\r\n" } else { "\n" };

        let mut lines = content.lines();
        let first_raw = lines.next()?;
        let first = strip_line_terminator(first_raw);
        if !first.starts_with('|') {
            return None;
        }
        let second_raw = lines.next()?;
        let second = strip_line_terminator(second_raw);
        if !is_table_separator(second) {
            return None;
        }
        Some(format!("{first}{line_ending}{second}{line_ending}"))
    }

    fn strip_line_terminator(line: &str) -> &str {
        line.strip_suffix("\r\n")
            .or_else(|| line.strip_suffix('\n'))
            .unwrap_or(line)
    }

    fn is_table_separator(line: &str) -> bool {
        if !line.starts_with('|') {
            return false;
        }
        let cells: Vec<&str> = line.split('|').filter(|c| !c.is_empty()).collect();
        if cells.is_empty() {
            return false;
        }
        cells.iter().all(|cell| is_separator_cell(cell))
    }

    fn is_separator_cell(cell: &str) -> bool {
        let s = cell.trim_matches(|c: char| c == ' ' || c == '\t');
        let s = s.strip_prefix(':').unwrap_or(s);
        if !s.starts_with('-') {
            return false;
        }
        let after_dashes = s.trim_start_matches('-');
        let dash_count = s.len() - after_dashes.len();
        if dash_count < 2 {
            return false;
        }
        matches!(after_dashes, "" | ":")
    }

    fn is_table_continuation(content: &str) -> bool {
        let trimmed = content.trim_start();
        if !trimmed.starts_with('|') {
            return false;
        }
        let mut lines = trimmed.lines();
        let first = lines.next().unwrap_or("").trim();
        if !first.starts_with('|') {
            return false;
        }
        let second = lines.next().unwrap_or("").trim();
        !is_table_separator(second)
    }

    let mut last_header: Option<String> = None;

    for chunk in chunks.iter_mut() {
        let content = &chunk.content;

        if let Some(header) = extract_table_header(content) {
            last_header = Some(header);
        } else if is_table_continuation(content) {
            if let Some(ref header) = last_header {
                chunk.content = format!("{header}{}", chunk.content);
            }
        } else {
            last_header = None;
        }
    }
}

/// Chunk text with explicit type specification.
///
/// This is a convenience function that constructs a ChunkingConfig from individual
/// parameters and calls `chunk_text`.
///
/// # Arguments
///
/// * `text` - The text to split into chunks
/// * `max_characters` - Maximum characters per chunk
/// * `overlap` - Character overlap between consecutive chunks
/// * `trim` - Whether to trim whitespace from boundaries
/// * `chunker_type` - Type of chunker to use (Text or Markdown)
///
/// # Returns
///
/// A ChunkingResult containing all chunks and their metadata.
///
/// # Examples
///
/// This function is `#[cfg(test)]`, so it does not exist in a normal build and the example
/// cannot be compiled. The public route is [`chunk_text`], which takes a
/// [`ChunkingConfig`](crate::chunking::ChunkingConfig) carrying the same knobs.
///
/// ```ignore
/// use xberg::chunking::{chunk_text_with_type, ChunkerType};
///
/// # fn example() -> xberg::Result<()> {
/// let result = chunk_text_with_type("Some text", 500, 50, true, ChunkerType::Text)?;
/// assert!(!result.chunks.is_empty());
/// # Ok(())
/// # }
/// ```
#[cfg(test)]
pub(crate) fn chunk_text_with_type(
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
        ..Default::default()
    };
    chunk_text(text, &config, None)
}

/// Batch process multiple texts with the same configuration.
///
/// This convenience function applies the same chunking configuration to multiple
/// texts in sequence.
///
/// # Arguments
///
/// * `texts` - Slice of text strings to chunk
/// * `config` - Chunking configuration to apply to all texts
///
/// # Returns
///
/// A vector of ChunkingResult objects, one per input text.
///
/// # Errors
///
/// Returns an error if chunking any individual text fails.
///
/// # Examples
///
/// This function is `#[cfg(test)]`, so it does not exist in a normal build and the example
/// cannot be compiled. The public route is [`chunk_text`], called once per input.
///
/// ```ignore
/// use xberg::chunking::{chunk_texts_batch, ChunkingConfig};
///
/// # fn example() -> xberg::Result<()> {
/// let config = ChunkingConfig::default();
/// let texts: Vec<String> = vec!["First text".to_string(), "Second text".to_string()];
/// let results = chunk_texts_batch(&texts, &config)?;
/// assert_eq!(results.len(), 2);
/// # Ok(())
/// # }
/// ```
#[cfg(test)]
pub(crate) fn chunk_texts_batch(texts: &[String], config: &ChunkingConfig) -> Result<Vec<ChunkingResult>> {
    texts.iter().map(|text| chunk_text(text, config, None)).collect()
}

#[cfg(test)]
mod tests;
