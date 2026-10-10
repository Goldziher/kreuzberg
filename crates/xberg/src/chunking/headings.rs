//! Heading extraction for Markdown chunk metadata.
//!
//! Parses markdown text to build a heading map, then resolves
//! which headings a chunk falls under based on its byte offset.

use super::boundaries::PageIndex;
use crate::types::{HeadingContext, HeadingLevel};
use pulldown_cmark::{Event, Options, Parser, TagEnd};

/// An entry in the heading map: `(byte_offset, level, text)`.
type HeadingEntry = (usize, u8, String);

/// Build a heading map from markdown text.
///
/// Entries are `(byte_offset, level, heading_text)` in document order.
pub(crate) fn build_heading_map(markdown: &str) -> HeadingMap {
    let parser = Parser::new_ext(markdown, Options::all());
    let mut headings = Vec::new();
    let mut current_heading: Option<(usize, u8)> = None;
    let mut heading_text = String::new();

    for (event, range) in parser.into_offset_iter() {
        match event {
            Event::Start(pulldown_cmark::Tag::Heading { level, .. }) => {
                current_heading = Some((range.start, heading_level_to_u8(level)));
                heading_text.clear();
            }
            Event::Text(text) if current_heading.is_some() => {
                heading_text.push_str(&text);
            }
            Event::Code(code) if current_heading.is_some() => {
                heading_text.push_str(&code);
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some((offset, level)) = current_heading.take() {
                    headings.push((offset, level, heading_text.clone()));
                }
            }
            _ => {}
        }
    }
    HeadingMap::new(headings)
}

/// Heading entries plus, for each, the index of its nearest preceding heading of a strictly
/// shallower level. Following those links from any heading yields exactly the open
/// h1 > h2 > ... stack at that point, so resolving a chunk costs O(depth) instead of a
/// walk over every earlier heading.
pub(crate) struct HeadingMap {
    entries: Vec<HeadingEntry>,
    parents: Vec<Option<usize>>,
}

impl HeadingMap {
    pub(crate) fn new(entries: Vec<HeadingEntry>) -> Self {
        let mut parents = Vec::with_capacity(entries.len());
        let mut open: Vec<usize> = Vec::new();
        for (index, &(_, level, _)) in entries.iter().enumerate() {
            while open.last().is_some_and(|&top| entries[top].1 >= level) {
                open.pop();
            }
            parents.push(open.last().copied());
            open.push(index);
        }
        Self { entries, parents }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Resolve the heading context for a chunk at the given byte offset.
///
/// Finds the headings that precede `byte_start` and builds a proper hierarchy stack
/// (h1 > h2 > h3, etc.).
///
/// When `page_boundaries` is provided, heading context is scoped to the page the
/// chunk starts on: headings declared on an *earlier* page are excluded from the
/// stack (#1289). Without this, a heading detected once (e.g. a document title on
/// page 1) would keep leaking into every later chunk for the rest of the combined
/// text — including onto pages belonging to an entirely different, concatenated
/// document that happens to carry no heading of its own. Headings declared on the
/// same page as the chunk still build the usual nested hierarchy. When
/// `page_boundaries` is `None` or empty (or the offset falls in no boundary), the
/// heading map is walked unrestricted, matching the pre-#1289 behavior.
pub(crate) fn resolve_heading_context(
    byte_start: usize,
    heading_map: &HeadingMap,
    pages: Option<&PageIndex<'_>>,
) -> Option<HeadingContext> {
    let page_start = pages.and_then(|pages| pages.page_start(byte_start));
    let entries = &heading_map.entries;

    let preceding = entries.partition_point(|&(offset, _, _)| offset <= byte_start);
    let mut cursor = preceding.checked_sub(1);
    let mut stack: Vec<HeadingLevel> = Vec::new();
    while let Some(index) = cursor {
        let (offset, level, ref text) = entries[index];
        if page_start.is_some_and(|page_start| offset < page_start) {
            break;
        }
        stack.push(HeadingLevel {
            level,
            text: text.clone(),
        });
        cursor = heading_map.parents[index];
    }
    stack.reverse();

    if stack.is_empty() {
        None
    } else {
        Some(HeadingContext { headings: stack })
    }
}

fn heading_level_to_u8(level: pulldown_cmark::HeadingLevel) -> u8 {
    match level {
        pulldown_cmark::HeadingLevel::H1 => 1,
        pulldown_cmark::HeadingLevel::H2 => 2,
        pulldown_cmark::HeadingLevel::H3 => 3,
        pulldown_cmark::HeadingLevel::H4 => 4,
        pulldown_cmark::HeadingLevel::H5 => 5,
        pulldown_cmark::HeadingLevel::H6 => 6,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::PageBoundary;

    fn resolve(byte_start: usize, map: &HeadingMap, pages: Option<&[PageBoundary]>) -> Option<HeadingContext> {
        let index = pages.map(|p| PageIndex::new(p).unwrap());
        resolve_heading_context(byte_start, map, index.as_ref())
    }

    /// The pre-index lookup: linear scans over every boundary.
    fn page_start_for_offset(byte_start: usize, page_boundaries: &[PageBoundary]) -> Option<usize> {
        page_boundaries
            .iter()
            .find(|b| byte_start >= b.byte_start && byte_start < b.byte_end)
            .or_else(|| page_boundaries.iter().rev().find(|b| b.byte_start <= byte_start))
            .map(|b| b.byte_start)
    }

    #[test]
    fn test_build_heading_map_basic() {
        let md = "# Title\n\nSome text.\n\n## Section 1\n\nContent.\n\n## Section 2\n\nMore content.";
        let map = build_heading_map(md);
        assert_eq!(map.entries.len(), 3);
        assert_eq!(map.entries[0], (0, 1, "Title".to_string()));
        assert_eq!(map.entries[1].1, 2);
        assert_eq!(map.entries[1].2, "Section 1");
        assert_eq!(map.entries[2].1, 2);
        assert_eq!(map.entries[2].2, "Section 2");
    }

    #[test]
    fn test_build_heading_map_nested() {
        let md = "# H1\n\n## H2\n\n### H3\n\nText.";
        let map = build_heading_map(md);
        assert_eq!(map.entries.len(), 3);
        assert_eq!(map.entries[0].1, 1);
        assert_eq!(map.entries[1].1, 2);
        assert_eq!(map.entries[2].1, 3);
    }

    #[test]
    fn test_build_heading_map_no_headings() {
        let md = "Just plain text without any headings.";
        let map = build_heading_map(md);
        assert!(map.is_empty());
    }

    #[test]
    fn test_build_heading_map_with_code_in_heading() {
        let md = "# Title with `code`\n\nText.";
        let map = build_heading_map(md);
        assert_eq!(map.entries.len(), 1);
        assert_eq!(map.entries[0].2, "Title with code");
    }

    #[test]
    fn test_resolve_heading_context_under_h2() {
        let map = HeadingMap::new(vec![
            (0, 1, "Title".to_string()),
            (10, 2, "Section A".to_string()),
            (30, 2, "Section B".to_string()),
        ]);
        let ctx = resolve(15, &map, None).unwrap();
        assert_eq!(ctx.headings.len(), 2);
        assert_eq!(ctx.headings[0].level, 1);
        assert_eq!(ctx.headings[0].text, "Title");
        assert_eq!(ctx.headings[1].level, 2);
        assert_eq!(ctx.headings[1].text, "Section A");
    }

    #[test]
    fn test_resolve_heading_context_root() {
        let map = HeadingMap::new(vec![(10, 1, "Title".to_string())]);
        let ctx = resolve(0, &map, None);
        assert!(ctx.is_none());
    }

    #[test]
    fn test_resolve_heading_context_superseded() {
        let map = HeadingMap::new(vec![
            (0, 1, "Title".to_string()),
            (10, 2, "Section A".to_string()),
            (20, 3, "Subsection".to_string()),
            (30, 2, "Section B".to_string()),
        ]);
        let ctx = resolve(35, &map, None).unwrap();
        assert_eq!(ctx.headings.len(), 2);
        assert_eq!(ctx.headings[1].text, "Section B");
    }

    #[test]
    fn test_resolve_heading_context_deep_nesting() {
        let map = HeadingMap::new(vec![
            (0, 1, "H1".to_string()),
            (5, 2, "H2".to_string()),
            (10, 3, "H3".to_string()),
            (15, 4, "H4".to_string()),
        ]);
        let ctx = resolve(20, &map, None).unwrap();
        assert_eq!(ctx.headings.len(), 4);
        assert_eq!(ctx.headings[3].level, 4);
    }

    /// Regression test for #1289: a heading declared on an earlier page must not
    /// leak into a later page's chunk when that later page carries no heading of
    /// its own — the classic symptom for concatenated documents (e.g. doc 1's
    /// title showing up on doc 2's / doc 3's un-headed pages).
    #[test]
    fn test_resolve_heading_context_resets_at_page_boundary_without_new_heading() {
        let map = HeadingMap::new(vec![(0, 1, "Doc 1 Title".to_string())]);
        let page_boundaries = [
            PageBoundary {
                page_number: 1,
                byte_start: 0,
                byte_end: 20,
            },
            PageBoundary {
                page_number: 2,
                byte_start: 20,
                byte_end: 40,
            },
            PageBoundary {
                page_number: 3,
                byte_start: 40,
                byte_end: 60,
            },
        ];

        let page_1_ctx = resolve(5, &map, Some(&page_boundaries));
        assert_eq!(
            page_1_ctx.unwrap().headings[0].text,
            "Doc 1 Title",
            "the chunk on the page the heading was declared on must still see it"
        );

        let page_2_ctx = resolve(25, &map, Some(&page_boundaries));
        assert!(
            page_2_ctx.is_none(),
            "page 2 has no heading of its own and must NOT inherit doc 1's title, got: {:?}",
            page_2_ctx
        );

        let page_3_ctx = resolve(45, &map, Some(&page_boundaries));
        assert!(
            page_3_ctx.is_none(),
            "page 3 has no heading of its own and must NOT inherit doc 1's title, got: {:?}",
            page_3_ctx
        );
    }

    /// A heading declared on the same page as the chunk (even mid-page, after
    /// other content) must still build the normal nested hierarchy.
    #[test]
    fn test_resolve_heading_context_same_page_hierarchy_still_builds() {
        let map = HeadingMap::new(vec![(20, 1, "Section".to_string()), (25, 2, "Subsection".to_string())]);
        let page_boundaries = [PageBoundary {
            page_number: 2,
            byte_start: 20,
            byte_end: 40,
        }];

        let ctx = resolve(30, &map, Some(&page_boundaries)).unwrap();
        assert_eq!(ctx.headings.len(), 2);
        assert_eq!(ctx.headings[0].text, "Section");
        assert_eq!(ctx.headings[1].text, "Subsection");
    }

    /// Without page boundary information, behavior is unchanged from pre-#1289:
    /// headings apply for the rest of the text regardless of any (absent) page
    /// structure.
    #[test]
    fn test_resolve_heading_context_no_page_boundaries_is_unrestricted() {
        let map = HeadingMap::new(vec![(0, 1, "Title".to_string())]);
        let ctx = resolve(1000, &map, None);
        assert_eq!(ctx.unwrap().headings[0].text, "Title");
    }

    /// The pre-index algorithm: replay every earlier heading through a stack.
    fn resolve_by_replay(
        byte_start: usize,
        entries: &[HeadingEntry],
        page_boundaries: Option<&[PageBoundary]>,
    ) -> Option<HeadingContext> {
        let page_start = page_boundaries.and_then(|boundaries| page_start_for_offset(byte_start, boundaries));
        let mut stack: Vec<HeadingLevel> = Vec::new();
        for &(offset, level, ref text) in entries {
            if offset > byte_start {
                break;
            }
            if page_start.is_some_and(|page_start| offset < page_start) {
                continue;
            }
            while stack.last().is_some_and(|h| h.level >= level) {
                stack.pop();
            }
            stack.push(HeadingLevel {
                level,
                text: text.clone(),
            });
        }
        (!stack.is_empty()).then_some(HeadingContext { headings: stack })
    }

    fn heading_dense_markdown(sections: usize) -> String {
        let mut md = String::new();
        for i in 0..sections {
            // Pseudo-random level walk exercises pops, equal levels, and level skips.
            let level = 1 + (i * 7 + i / 3) % 6;
            md.push_str(&format!(
                "{} Heading {i}\n\nBody text for section {i}.\n\n",
                "#".repeat(level)
            ));
        }
        md
    }

    #[test]
    fn test_resolve_heading_context_matches_replay_for_every_offset() {
        let md = heading_dense_markdown(300);
        let map = build_heading_map(&md);
        let boundaries: Vec<PageBoundary> = (0..md.len().div_ceil(500))
            .map(|page| PageBoundary {
                page_number: page as u32 + 1,
                byte_start: page * 500,
                byte_end: ((page + 1) * 500).min(md.len()),
            })
            .collect();
        for byte_start in (0..=md.len()).step_by(7) {
            for pages in [None, Some(boundaries.as_slice())] {
                assert_eq!(
                    format!("{:?}", resolve(byte_start, &map, pages)),
                    format!("{:?}", resolve_by_replay(byte_start, &map.entries, pages)),
                    "mismatch at byte {byte_start}, paged={}",
                    pages.is_some()
                );
            }
        }
    }

    #[test]
    fn test_resolve_heading_context_same_level_headings_resolve_one_deep() {
        let map = HeadingMap::new((0..1000).map(|i| (i * 10, 2, format!("H{i}"))).collect());
        let ctx = resolve(9_995, &map, None).unwrap();
        assert_eq!(ctx.headings.len(), 1);
        assert_eq!(ctx.headings[0].text, "H999");
    }

    /// Replaying every earlier heading per chunk was quadratic: 50k same-level headings
    /// resolved at 50k offsets is ~1.25e9 replay steps, minutes in debug; linked lookup is instant.
    #[test]
    fn test_resolve_heading_context_scales_linearly_in_heading_count() {
        let map = HeadingMap::new((0..50_000).map(|i| (i * 10, 2, format!("H{i}"))).collect());
        let started = std::time::Instant::now();
        for i in 0..50_000 {
            let ctx = resolve(i * 10 + 5, &map, None).unwrap();
            assert_eq!(ctx.headings.len(), 1);
        }
        assert!(
            started.elapsed() < std::time::Duration::from_secs(5),
            "heading resolution took {:?}",
            started.elapsed()
        );
    }

    /// End to end: every chunk's heading context from `chunk_text` equals the replay result.
    #[test]
    fn test_chunk_text_heading_context_matches_replay_on_heading_dense_markdown() {
        use crate::chunking::core::chunk_text;
        use crate::core::config::processing::{ChunkerType, ChunkingConfig};

        let md = heading_dense_markdown(400);
        let config = ChunkingConfig {
            max_characters: 300,
            overlap: 50,
            chunker_type: ChunkerType::Markdown,
            ..Default::default()
        };
        let entries = build_heading_map(&md).entries;
        let result = chunk_text(&md, &config, None).unwrap();
        assert!(result.chunks.len() > 50);
        for chunk in &result.chunks {
            assert_eq!(
                format!("{:?}", chunk.metadata.heading_context),
                format!("{:?}", resolve_by_replay(chunk.metadata.byte_start, &entries, None)),
                "chunk at byte {}",
                chunk.metadata.byte_start
            );
        }
    }
}
