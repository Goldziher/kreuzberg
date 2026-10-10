//! Straightforward reference implementations of the metadata the pipeline derives for each chunk.
//!
//! These intentionally use the simplest possible algorithms (linear scans, full replays) so the
//! optimized indexed paths can be checked against them.

use pulldown_cmark::{Event, HeadingLevel as MdLevel, Options, Parser, Tag, TagEnd};
use xberg::types::{HeadingContext, HeadingLevel, PageBoundary};

pub fn heading_entries(markdown: &str) -> Vec<(usize, u8, String)> {
    let mut entries = Vec::new();
    let mut current: Option<(usize, u8)> = None;
    let mut text = String::new();
    for (event, range) in Parser::new_ext(markdown, Options::all()).into_offset_iter() {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                let level = match level {
                    MdLevel::H1 => 1,
                    MdLevel::H2 => 2,
                    MdLevel::H3 => 3,
                    MdLevel::H4 => 4,
                    MdLevel::H5 => 5,
                    MdLevel::H6 => 6,
                };
                current = Some((range.start, level));
                text.clear();
            }
            Event::Text(t) if current.is_some() => text.push_str(&t),
            Event::Code(c) if current.is_some() => text.push_str(&c),
            Event::End(TagEnd::Heading(_)) => {
                if let Some((offset, level)) = current.take() {
                    entries.push((offset, level, text.clone()));
                }
            }
            _ => {}
        }
    }
    entries
}

fn page_start_for_offset(offset: usize, pages: &[PageBoundary]) -> Option<usize> {
    pages
        .iter()
        .find(|b| offset >= b.byte_start && offset < b.byte_end)
        .or_else(|| pages.iter().rev().find(|b| b.byte_start <= offset))
        .map(|b| b.byte_start)
}

pub fn heading_context(
    entries: &[(usize, u8, String)],
    byte_start: usize,
    pages: Option<&[PageBoundary]>,
) -> Option<HeadingContext> {
    let page_start = pages.and_then(|p| page_start_for_offset(byte_start, p));
    let mut stack: Vec<HeadingLevel> = Vec::new();
    for (offset, level, text) in entries {
        if *offset > byte_start {
            break;
        }
        if page_start.is_some_and(|start| *offset < start) {
            continue;
        }
        while stack.last().is_some_and(|h| h.level >= *level) {
            stack.pop();
        }
        stack.push(HeadingLevel {
            level: *level,
            text: text.clone(),
        });
    }
    (!stack.is_empty()).then_some(HeadingContext { headings: stack })
}

/// `(first_page, last_page, pages)` for every boundary overlapping `[start, end)`.
pub fn page_range(start: usize, end: usize, pages: &[PageBoundary]) -> (Option<u32>, Option<u32>, Vec<u32>) {
    let hit: Vec<u32> = pages
        .iter()
        .filter(|b| start < b.byte_end && end > b.byte_start)
        .map(|b| b.page_number)
        .collect();
    (hit.first().copied(), hit.last().copied(), hit)
}
