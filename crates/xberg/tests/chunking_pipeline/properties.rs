//! Correctness properties that must hold for every chunker over every corpus document.

use icu_segmenter::GraphemeClusterSegmenter;
use xberg::chunking::{ChunkerType, ChunkingConfig};
use xberg::types::{ChunkMetadata, HeadingContext, PageBoundary};

use crate::corpus::{self, SHAPES};
use crate::digest::{CAPACITIES, CHUNKERS, config, run};
use crate::fixtures;
use crate::oracle;

/// Named documents every property test runs over: real corpus files plus one of each generated shape.
pub fn documents() -> Vec<(String, String)> {
    let mut docs = fixtures::real_documents();
    for (name, generator) in SHAPES {
        docs.push((format!("gen/{name}"), generator(40 * 1024, 11)));
    }
    docs
}

pub fn grapheme_count(text: &str) -> usize {
    GraphemeClusterSegmenter::new()
        .segment_str(text)
        .count()
        .saturating_sub(1)
}

fn flatten(context: Option<&HeadingContext>) -> Option<Vec<(u8, &str)>> {
    context.map(|c| c.headings.iter().map(|h| (h.level, h.text.as_str())).collect())
}

fn is_blank(text: &str) -> bool {
    text.chars().all(char::is_whitespace)
}

pub fn assert_structure(name: &str, text: &str, cfg: &ChunkingConfig, pages: Option<&[PageBoundary]>) {
    let result = run(text, cfg, pages);
    let semantic = cfg.chunker_type == ChunkerType::Semantic;
    let label = format!(
        "{name} {:?} max={} overlap={}",
        cfg.chunker_type, cfg.max_characters, cfg.overlap
    );
    assert_eq!(result.chunk_count, result.chunks.len(), "{label}: chunk_count");

    let mut previous: Option<&ChunkMetadata> = None;
    for (index, chunk) in result.chunks.iter().enumerate() {
        let meta = &chunk.metadata;
        assert_eq!(meta.chunk_index, index, "{label}: chunk_index");
        assert_eq!(meta.total_chunks, result.chunks.len(), "{label}: total_chunks");
        assert!(!chunk.content.is_empty(), "{label}: chunk {index} is empty");
        assert!(
            meta.byte_start <= meta.byte_end && meta.byte_end <= text.len(),
            "{label}: chunk {index} offsets"
        );

        let size = chunk.content.chars().count();
        if size > cfg.max_characters {
            assert!(
                grapheme_count(&chunk.content) <= 1,
                "{label}: chunk {index} has {size} chars (> {}) and is not a single grapheme",
                cfg.max_characters
            );
        }

        if let Some(prev) = previous {
            assert!(
                meta.byte_start >= prev.byte_start,
                "{label}: chunk {index} starts before its predecessor"
            );
        }

        // The semantic chunker reports approximate offsets for groups it re-splits and prepends
        // overlap text to chunk content, so only the splitter-backed chunkers keep content == span.
        if !semantic {
            let slice = text
                .get(meta.byte_start..meta.byte_end)
                .unwrap_or_else(|| panic!("{label}: chunk {index} offsets are not char boundaries"));
            assert_eq!(
                chunk.content, slice,
                "{label}: chunk {index} content differs from its source span"
            );
            match previous {
                None => assert!(
                    is_blank(&text[..meta.byte_start]),
                    "{label}: text before first chunk is not blank"
                ),
                Some(prev) => {
                    assert!(meta.byte_end > prev.byte_end, "{label}: chunk {index} does not advance");
                    if meta.byte_start >= prev.byte_end {
                        assert!(
                            is_blank(&text[prev.byte_end..meta.byte_start]),
                            "{label}: non-whitespace lost between chunks {} and {index}",
                            index - 1
                        );
                    } else {
                        let overlap_chars = text[meta.byte_start..prev.byte_end].chars().count();
                        assert!(
                            overlap_chars <= cfg.overlap,
                            "{label}: chunks {} and {index} overlap by {overlap_chars} chars (> {})",
                            index - 1,
                            cfg.overlap
                        );
                    }
                }
            }
        }
        previous = Some(meta);

        if let Some(boundaries) = pages {
            let (first, last, hit) = oracle::page_range(meta.byte_start, meta.byte_end, boundaries);
            assert_eq!(
                (meta.first_page, meta.last_page),
                (first, last),
                "{label}: page range of chunk {index}"
            );
            let spans: Vec<u32> = meta.page_spans.iter().map(|s| s.page).collect();
            assert_eq!(spans, hit, "{label}: page spans of chunk {index}");
        }
    }
    if let Some(last) = previous
        && !semantic
    {
        assert!(
            is_blank(&text[last.byte_end..]),
            "{label}: text after last chunk is not blank"
        );
    }
    if semantic && cfg.overlap == 0 {
        let kept: String = result
            .chunks
            .iter()
            .flat_map(|c| c.content.chars())
            .filter(|c| !c.is_whitespace())
            .collect();
        let original: String = text.chars().filter(|c| !c.is_whitespace()).collect();
        assert!(
            kept == original,
            "{label}: semantic chunks lost or reordered non-whitespace text"
        );
    }

    if matches!(cfg.chunker_type, ChunkerType::Markdown | ChunkerType::Semantic) {
        let entries = oracle::heading_entries(text);
        for (index, chunk) in result.chunks.iter().enumerate() {
            let expected = oracle::heading_context(&entries, chunk.metadata.byte_start, pages);
            assert_eq!(
                flatten(chunk.metadata.heading_context.as_ref()),
                flatten(expected.as_ref()),
                "{label}: heading_context of chunk {index}"
            );
        }
    } else {
        assert!(
            result.chunks.iter().all(|c| c.metadata.heading_context.is_none()),
            "{label}: text chunker must not attach heading context"
        );
    }
}

#[test]
fn every_chunker_preserves_structure_on_the_corpus() {
    for (name, text) in documents() {
        for (_, chunker) in CHUNKERS {
            for &(max, overlap) in CAPACITIES {
                assert_structure(&name, &text, &config(*chunker, max, overlap), None);
            }
        }
    }
}

#[test]
fn every_chunker_preserves_structure_with_pages() {
    for (name, text) in documents() {
        let contiguous = corpus::pages(&text, 1_500);
        let gapped = fixtures::gapped_pages(&text, 1_100);
        for (_, chunker) in CHUNKERS {
            for &(max, overlap) in &CAPACITIES[1..3] {
                assert_structure(&name, &text, &config(*chunker, max, overlap), Some(&contiguous));
                assert_structure(&name, &text, &config(*chunker, max, overlap), Some(&gapped));
            }
        }
    }
}

#[test]
fn seeded_random_documents_and_capacities_preserve_structure() {
    let mut rng = corpus::Rng::new(0x5eed);
    for round in 0..60 {
        let (shape_name, generator) = SHAPES[rng.below(SHAPES.len())];
        let text = generator(rng.range(2_000, 24_000), rng.next_u64());
        let max = rng.range(20, 3_000);
        let overlap = if rng.below(2) == 0 { 0 } else { rng.below(max.min(400)) };
        let (_, chunker) = CHUNKERS[rng.below(CHUNKERS.len())];
        let pages = (rng.below(2) == 0).then(|| corpus::pages(&text, rng.range(200, 4_000)));
        assert_structure(
            &format!("random#{round}/{shape_name}"),
            &text,
            &config(chunker, max, overlap),
            pages.as_deref(),
        );
    }
}
