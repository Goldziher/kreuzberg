//! Optimality of the split points, not just their validity.
//!
//! The splitter picks, at each chunk start, the coarsest semantic level whose first unit fits,
//! then packs whole units of that level. Chunks are therefore *not* greedy-maximal across
//! paragraph or heading boundaries (a short section is not merged with the next one when the
//! next one is a separate unit); that is deliberate and is not asserted here. What must hold:
//! the cut never falls inside a unit that would have fitted, uniform text packs perfectly, and
//! overlap is exactly as wide as the configuration allows.

use std::fmt::Write as _;

use xberg::chunking::ChunkerType;

use crate::corpus::{self, Generator};
use crate::digest::{config, run};
use crate::fixtures;

fn is_word(c: char) -> bool {
    c.is_ascii_alphanumeric()
}

fn char_len(s: &str) -> usize {
    s.chars().count()
}

/// Prose-like shapes whose words are ASCII, so word boundaries are unambiguous.
const ASCII_SHAPES: &[(&str, Generator)] = &[
    ("flat_prose", corpus::flat_prose),
    ("one_long_line", corpus::one_long_line),
    ("csv_like", corpus::csv_like),
    ("json_like", corpus::json_like),
    ("mixed_markdown", corpus::mixed_markdown),
    ("nested_lists", corpus::nested_lists),
];

#[test]
fn a_chunk_ends_inside_a_word_only_when_the_rest_of_that_word_cannot_fit() {
    for (shape, generator) in ASCII_SHAPES {
        let text = generator(60 * 1024, 5);
        for chunker in [ChunkerType::Text, ChunkerType::Markdown] {
            for max in [30usize, 120, 500, 2_000] {
                let result = run(&text, &config(chunker, max, 0), None);
                for pair in result.chunks.windows(2) {
                    let (a, b) = (&pair[0].metadata, &pair[1].metadata);
                    if b.byte_start != a.byte_end {
                        continue;
                    }
                    let before = text[..a.byte_end].chars().next_back().unwrap();
                    let after = text[b.byte_start..].chars().next().unwrap();
                    if !(is_word(before) && is_word(after)) {
                        continue;
                    }
                    let word_end = text[a.byte_start..]
                        .find(|c: char| !is_word(c))
                        .map_or(text.len(), |i| a.byte_start + i);
                    let rest = char_len(&text[a.byte_start..word_end]);
                    assert!(
                        rest > max,
                        "{shape} {chunker:?} max={max}: chunk at {} was cut inside a word whose remainder ({rest} chars) fits",
                        a.byte_start
                    );
                }
            }
        }
    }
}

#[test]
fn a_sentence_that_fits_is_never_split_in_prose() {
    let text = corpus::flat_prose(80 * 1024, 5);
    for chunker in [ChunkerType::Text, ChunkerType::Markdown] {
        for max in [60usize, 120, 500, 2_000] {
            let result = run(&text, &config(chunker, max, 0), None);
            for chunk in &result.chunks {
                let start = chunk.metadata.byte_start;
                let first_sentence_end = text[start..]
                    .char_indices()
                    .find(|&(i, c)| {
                        matches!(c, '.' | '!' | '?')
                            && text[start + i + 1..].chars().next().is_none_or(char::is_whitespace)
                    })
                    .map_or(text.len(), |(i, c)| start + i + c.len_utf8());
                let first_sentence = &text[start..first_sentence_end];
                if first_sentence.chars().any(char::is_alphanumeric) && char_len(first_sentence) <= max {
                    assert!(
                        chunk.metadata.byte_end >= first_sentence_end,
                        "{chunker:?} max={max}: chunk at {start} splits a sentence that fits"
                    );
                    let last = chunk.content.trim_end().chars().next_back().unwrap();
                    let ends_at_boundary = matches!(last, '.' | '!' | '?') || chunk.metadata.byte_end == text.len();
                    assert!(
                        ends_at_boundary,
                        "{chunker:?} max={max}: chunk at {start} ends mid-sentence ({last:?}) although its first sentence fits"
                    );
                }
            }
        }
    }
}

#[test]
fn fenced_code_table_rows_and_headings_are_not_cut_when_they_fit() {
    let fenced = corpus::code_fences(120 * 1024, 5);
    let mut fences: Vec<(usize, usize)> = Vec::new();
    let mut open = None;
    let mut position = 0;
    for line in fenced.split_inclusive('\n') {
        if line.starts_with("```") {
            match open.take() {
                Some(start) => fences.push((start, position + line.len())),
                None => open = Some(position),
            }
        }
        position += line.len();
    }
    for max in [300usize, 500, 1_000, 4_000] {
        let result = run(&fenced, &config(ChunkerType::Markdown, max, 0), None);
        for pair in result.chunks.windows(2) {
            let (cut, next) = (pair[0].metadata.byte_end, pair[1].metadata.byte_start);
            for &(start, end) in &fences {
                if cut > start && next < end {
                    assert!(
                        char_len(&fenced[start..end]) > max,
                        "max={max}: a code fence of {} chars was cut at {cut}",
                        char_len(&fenced[start..end])
                    );
                }
            }
        }
    }

    let table = corpus::long_table(60 * 1024, 5);
    for max in [100usize, 500, 2_000] {
        let result = run(&table, &config(ChunkerType::Markdown, max, 0), None);
        for pair in result.chunks.windows(2) {
            let (cut, next) = (pair[0].metadata.byte_end, pair[1].metadata.byte_start);
            let at_row_end = table[..cut].ends_with('\n') || table[cut..next].contains('\n');
            assert!(at_row_end, "max={max}: table row cut at {cut}");
        }
    }

    let headings = corpus::heading_dense(60 * 1024, 5);
    for max in [100usize, 500, 2_000] {
        let result = run(&headings, &config(ChunkerType::Markdown, max, 0), None);
        for chunk in &result.chunks {
            let start = chunk.metadata.byte_start;
            let line_start = headings[..start].rfind('\n').map_or(0, |i| i + 1);
            let line_end = headings[start..].find('\n').map_or(headings.len(), |i| start + i);
            let line = &headings[line_start..line_end];
            assert!(
                !(line.starts_with('#') && start > line_start && char_len(line) <= max),
                "max={max}: a chunk starts inside the heading line {line:?}"
            );
        }
    }
}

fn uniform_tokens(count: usize) -> String {
    let mut out = String::new();
    for i in 0..count {
        write!(out, "tok{i:03} ").unwrap();
    }
    out.truncate(out.trim_end().len());
    out
}

const TOKEN_CHARS: usize = 7;

fn tokens_in(s: &str) -> usize {
    s.split_whitespace().count()
}

#[test]
fn uniform_text_packs_into_the_minimum_number_of_chunks() {
    let total = 997;
    let text = uniform_tokens(total);
    for chunker in [ChunkerType::Text, ChunkerType::Markdown] {
        for max in [7usize, 20, 50, 100, 333, 1_000, 5_000] {
            let per_chunk = ((max + 1) / TOKEN_CHARS).max(1);
            let result = run(&text, &config(chunker, max, 0), None);
            assert_eq!(
                result.chunks.len(),
                total.div_ceil(per_chunk),
                "{chunker:?} max={max}: not the minimal chunk count for {per_chunk} tokens per chunk"
            );
            let counts: Vec<usize> = result.chunks.iter().map(|c| tokens_in(&c.content)).collect();
            let (last, full) = counts.split_last().unwrap();
            assert!(
                full.iter().all(|&n| n == per_chunk),
                "{chunker:?} max={max}: a non-final chunk is underfilled: {counts:?}"
            );
            assert_eq!(
                *last,
                total - per_chunk * full.len(),
                "{chunker:?} max={max}: trailing chunk size"
            );
        }
    }
}

#[test]
fn overlap_is_as_wide_as_the_configuration_allows() {
    let total = 600;
    let text = uniform_tokens(total);
    for chunker in [ChunkerType::Text, ChunkerType::Markdown] {
        for (max, overlap) in [
            (100usize, 20usize),
            (100, 50),
            (333, 100),
            (1_000, 300),
            (1_000, 6),
            (100, 7),
        ] {
            let per_chunk = (max + 1) / TOKEN_CHARS;
            let overlap_tokens = (overlap + 1) / TOKEN_CHARS;
            let result = run(&text, &config(chunker, max, overlap), None);
            for pair in result.chunks.windows(2) {
                let (a, b) = (&pair[0].metadata, &pair[1].metadata);
                let shared = if b.byte_start < a.byte_end {
                    tokens_in(&text[b.byte_start..a.byte_end])
                } else {
                    0
                };
                assert_eq!(
                    shared, overlap_tokens,
                    "{chunker:?} max={max} overlap={overlap}: consecutive chunks share {shared} tokens, expected {overlap_tokens}"
                );
            }
            let counts: Vec<usize> = result.chunks.iter().map(|c| tokens_in(&c.content)).collect();
            assert!(
                counts[..counts.len() - 1].iter().all(|&n| n == per_chunk),
                "{chunker:?} max={max} overlap={overlap}: overlap must not shrink chunks: {counts:?}"
            );
        }
    }
}

#[test]
fn chunk_count_stays_within_twice_the_capacity_lower_bound_on_prose() {
    for (shape, generator) in ASCII_SHAPES {
        let text = generator(80 * 1024, 5);
        let content_chars = text.chars().filter(|c| !c.is_whitespace()).count();
        for chunker in [ChunkerType::Text, ChunkerType::Markdown] {
            for max in [120usize, 500, 2_000] {
                let count = run(&text, &config(chunker, max, 0), None).chunks.len();
                let lower_bound = content_chars.div_ceil(max);
                assert!(
                    count >= lower_bound,
                    "{shape} {chunker:?} max={max}: {count} chunks cannot hold {content_chars} chars"
                );
                assert!(
                    count <= 2 * lower_bound,
                    "{shape} {chunker:?} max={max}: {count} chunks is more than twice the lower bound {lower_bound}"
                );
            }
        }
    }
}

#[test]
fn heading_context_names_exactly_the_open_headings() {
    let text = "# Guide\n\nIntro text.\n\n## Setup\n\nInstall it.\n\n### Linux\n\nUse apt.\n\n### macOS\n\nUse brew.\n\n## Usage\n\nRun it.\n\n# Appendix\n\nExtra.\n";
    let result = run(text, &config(ChunkerType::Markdown, 20, 0), None);
    let mut seen = Vec::new();
    for chunk in &result.chunks {
        let path: Vec<&str> = chunk
            .metadata
            .heading_context
            .as_ref()
            .map(|c| c.headings.iter().map(|h| h.text.as_str()).collect())
            .unwrap_or_default();
        seen.push((chunk.content.clone(), path.join(" > ")));
    }
    let expectations = [
        ("# Guide", "Guide"),
        ("Intro text.", "Guide"),
        ("## Setup", "Guide > Setup"),
        ("Install it.", "Guide > Setup"),
        ("### Linux", "Guide > Setup > Linux"),
        ("Use apt.", "Guide > Setup > Linux"),
        ("### macOS", "Guide > Setup > macOS"),
        ("Use brew.", "Guide > Setup > macOS"),
        ("## Usage", "Guide > Usage"),
        ("Run it.", "Guide > Usage"),
        ("# Appendix", "Appendix"),
        ("Extra.", "Appendix"),
    ];
    for (first_line, expected) in expectations {
        let (_, path) = seen
            .iter()
            .find(|(content, _)| content.lines().any(|line| line == first_line))
            .unwrap_or_else(|| panic!("no chunk contains the line {first_line:?}: {seen:?}"));
        assert_eq!(path, expected, "heading context of the chunk containing {first_line:?}");
    }
}

#[test]
fn real_documents_never_cut_inside_a_fitting_word() {
    for (name, text) in fixtures::real_documents() {
        for max in [120usize, 500] {
            let result = run(&text, &config(ChunkerType::Text, max, 0), None);
            for pair in result.chunks.windows(2) {
                let (a, b) = (&pair[0].metadata, &pair[1].metadata);
                if b.byte_start != a.byte_end {
                    continue;
                }
                let before = text[..a.byte_end].chars().next_back().unwrap();
                let after = text[b.byte_start..].chars().next().unwrap();
                if is_word(before) && is_word(after) {
                    let word_end = text[a.byte_start..]
                        .find(|c: char| !is_word(c))
                        .map_or(text.len(), |i| a.byte_start + i);
                    assert!(
                        char_len(&text[a.byte_start..word_end]) > max,
                        "{name} max={max}: cut inside a fitting word at {}",
                        a.byte_end
                    );
                }
            }
        }
    }
}

/// When a sentence is too long for one chunk and the cut lands just before its closing
/// punctuation, sentence segmentation restarted at the new chunk start used to see the lone
/// `"! "` as a complete sentence and emit it as its own chunk.
#[test]
fn prose_has_no_punctuation_only_orphan_chunks() {
    let text = corpus::flat_prose(80 * 1024, 11);
    for chunker in [ChunkerType::Text, ChunkerType::Markdown] {
        for max in [60usize, 120, 500] {
            let result = run(&text, &config(chunker, max, 0), None);
            for chunk in &result.chunks {
                assert!(
                    chunk.content.chars().any(char::is_alphanumeric),
                    "{chunker:?} max={max}: orphan chunk {:?} at {}",
                    chunk.content,
                    chunk.metadata.byte_start
                );
            }
        }
    }
}
