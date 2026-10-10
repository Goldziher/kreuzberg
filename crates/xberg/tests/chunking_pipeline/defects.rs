//! Regression tests for splitting defects found while building the corpus tests.

use xberg::chunking::ChunkerType;

use crate::corpus;
use crate::digest::{CAPACITIES, config, run};
use crate::fixtures;
use crate::oracle;
use crate::properties::documents;

/// The semantic chunker used to interpolate `byte_start`/`byte_end` proportionally for any
/// topic group it had to re-split, so the offsets neither located the chunk's text nor
/// necessarily fell on a character boundary.
#[test]
fn semantic_chunk_offsets_locate_the_chunk_text_without_overlap() {
    for (name, text) in documents() {
        for &(max, _) in CAPACITIES {
            let result = run(&text, &config(ChunkerType::Semantic, max, 0), None);
            for (index, chunk) in result.chunks.iter().enumerate() {
                let meta = &chunk.metadata;
                let span = text.get(meta.byte_start..meta.byte_end).unwrap_or_else(|| {
                    panic!(
                        "{name} max={max}: chunk {index} offsets {}..{} are not char boundaries",
                        meta.byte_start, meta.byte_end
                    )
                });
                assert_eq!(
                    chunk.content, span,
                    "{name} max={max}: chunk {index} offsets do not locate its content"
                );
            }
        }
    }
}

fn section_leaf(content: &str) -> &str {
    let heading = content.lines().next().unwrap().trim_start_matches("# ");
    heading.rsplit(" > ").next().unwrap()
}

/// YAML/JSON sections used to carry `0..0`, so every chunk resolved to the first page and no
/// offset located the section in the source.
#[test]
fn yaml_and_json_sections_report_their_source_range_and_pages() {
    let yaml = corpus::yaml_like(40 * 1024, 3);
    let json = fixtures::json_object(40 * 1024, 3);
    for (name, text) in [("yaml", &yaml), ("json", &json)] {
        let contiguous = corpus::pages(text, 700);
        let gapped = fixtures::gapped_pages(text, 600);
        for (layout, pages) in [("contiguous", &contiguous), ("gapped", &gapped)] {
            let result = run(text, &config(ChunkerType::Yaml, 100_000, 0), Some(pages));
            assert!(result.chunks.len() > 20, "{name}: expected many sections");
            let mut distinct_pages = std::collections::BTreeSet::new();
            for chunk in &result.chunks {
                let meta = &chunk.metadata;
                let span = text
                    .get(meta.byte_start..meta.byte_end)
                    .unwrap_or_else(|| panic!("{name}: span is not on char boundaries"));
                assert!(
                    span.contains(section_leaf(&chunk.content)),
                    "{name} {layout}: span {span:?} does not hold section {:?}",
                    chunk.content.lines().next()
                );
                let (first, last, hit) = oracle::page_range(meta.byte_start, meta.byte_end, pages);
                assert_eq!(
                    (meta.first_page, meta.last_page),
                    (first, last),
                    "{name} {layout}: page range"
                );
                let spans: Vec<u32> = meta.page_spans.iter().map(|s| s.page).collect();
                assert_eq!(spans, hit, "{name} {layout}: page spans");
                distinct_pages.extend(first);
            }
            assert!(
                distinct_pages.len() > 10,
                "{name} {layout}: sections should spread across pages, got {distinct_pages:?}"
            );
        }
    }
}

/// A list bullet, heading marker or closing mark followed by a unit too long to join it used to
/// be emitted as a chunk of its own.
#[test]
fn bullets_heading_markers_and_closing_marks_are_never_chunks_of_their_own() {
    let long_link =
        "[a link whose text alone is longer than the whole chunk budget allows](https://example.com/some/long/path)";
    let mut text = String::new();
    for i in 0..60 {
        text.push_str(&format!("- {long_link} item {i}\n"));
        text.push_str(&format!(
            "## Heading number {i} with a title that is quite long indeed\n\n"
        ));
        text.push_str("A sentence that ends right at the budget edge of a chunk, ok!\n\n");
    }
    for chunker in [ChunkerType::Markdown, ChunkerType::Text] {
        for max in [30usize, 60, 80] {
            let result = run(&text, &config(chunker, max, 0), None);
            for chunk in &result.chunks[..result.chunks.len() - 1] {
                assert!(
                    chunk.content.chars().any(char::is_alphanumeric),
                    "{chunker:?} max={max}: chunk {:?} at {} carries no content",
                    chunk.content,
                    chunk.metadata.byte_start
                );
            }
        }
    }
}
