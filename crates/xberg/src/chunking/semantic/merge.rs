//! Segment merging for semantic chunking.
//!
//! Groups segments by topic boundaries, merges within groups, and falls back
//! to `text_splitter::TextSplitter` when a merged group exceeds the budget.

use crate::chunking::text_splitter::TextSplitter;

/// A text segment with its byte offset in the original document.
#[derive(Debug, Clone, Copy)]
pub struct Segment<'a> {
    /// Borrowed text slice from the original document.
    pub text: &'a str,
    /// Byte offset of the start of this segment in the original document.
    pub byte_start: usize,
}

/// A merged chunk produced by `merge_segments`.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergedChunk {
    /// Merged text content (may include overlap from the previous chunk).
    pub text: String,
    /// Byte offset of the first character in the original document.
    pub byte_start: usize,
    /// Byte offset one past the last character in the original document.
    pub byte_end: usize,
}

/// Extract up to `n` characters from the end of `s`, respecting UTF-8 boundaries.
fn tail_chars(s: &str, n: usize) -> &str {
    if n == 0 {
        return "";
    }
    let total = s.chars().count();
    if n >= total {
        return s;
    }
    let skip = total - n;
    let byte_offset = s.char_indices().nth(skip).map(|(i, _)| i).unwrap_or(0);
    &s[byte_offset..]
}

/// Prepend overlap text from the previous group, staying within the ceiling.
fn prepend_overlap(group_text: &str, prev_tail: &str, overlap: usize, ceiling: usize) -> String {
    let tail = tail_chars(prev_tail, overlap);
    if tail.is_empty() {
        return group_text.to_string();
    }

    let candidate = format!("{tail}\n\n{group_text}");
    if candidate.chars().count() <= ceiling {
        return candidate;
    }

    let group_chars = group_text.chars().count();
    let available = ceiling.saturating_sub(group_chars + 2);
    if available > 0 {
        let truncated = tail_chars(prev_tail, available);
        format!("{truncated}\n\n{group_text}")
    } else {
        group_text.to_string()
    }
}

/// Merge segments into chunks guided by topic boundaries.
///
/// # Arguments
///
/// * `source_text` – the original document text that segments reference.
/// * `segments` – ordered text segments (subslices of `source_text`).
/// * `boundaries` – parallel bool slice; `true` at index `i` means segment `i`
///   starts a new topic group.
/// * `max_characters` – maximum characters per output chunk.
/// * `overlap` – number of characters from the tail of the previous group to
///   prepend to the next group's first chunk.
///
/// # Panics (debug)
///
/// Debug-asserts that `segments.len() == boundaries.len()`.
pub(crate) fn merge_segments(
    source_text: &str,
    segments: &[Segment<'_>],
    boundaries: &[bool],
    max_characters: usize,
    overlap: usize,
) -> Vec<MergedChunk> {
    if segments.is_empty() {
        return Vec::new();
    }
    debug_assert_eq!(
        segments.len(),
        boundaries.len(),
        "segments and boundaries must have the same length"
    );

    let mut groups: Vec<std::ops::Range<usize>> = Vec::new();
    let mut group_start = 0;
    for (i, &is_boundary) in boundaries.iter().enumerate().skip(1) {
        if is_boundary {
            groups.push(group_start..i);
            group_start = i;
        }
    }
    groups.push(group_start..segments.len());

    let mut result: Vec<MergedChunk> = Vec::new();
    let mut prev_group_tail: Option<String> = None;

    for group in &groups {
        let group_segments = &segments[group.clone()];

        let group_byte_start = group_segments.first().unwrap().byte_start;
        let last_seg = group_segments.last().unwrap();
        let group_byte_end = last_seg.byte_start + last_seg.text.len();

        let group_text = &source_text[group_byte_start..group_byte_end];

        if group_text.chars().count() <= max_characters {
            let text = match (overlap > 0, &prev_group_tail) {
                (true, Some(prev_tail)) => prepend_overlap(group_text, prev_tail, overlap, max_characters),
                _ => group_text.to_string(),
            };

            result.push(MergedChunk {
                text,
                byte_start: group_byte_start,
                byte_end: group_byte_end,
            });
        } else {
            let splitter = TextSplitter::new(max_characters);
            let group_origin = group_text.as_ptr() as usize;

            for chunk_text in splitter.chunks(group_text) {
                // The splitter yields subslices of `group_text`, so pointer distance is the exact
                // byte offset; the trimmed chunk's own span is what the offsets must describe.
                let byte_start = group_byte_start + (chunk_text.as_ptr() as usize - group_origin);

                result.push(MergedChunk {
                    text: chunk_text.to_string(),
                    byte_start,
                    byte_end: byte_start + chunk_text.len(),
                });
            }
        }

        prev_group_tail = Some(group_text.to_string());
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_segment_under_budget() {
        let source = "hello world";
        let segments = [Segment {
            text: source,
            byte_start: 0,
        }];
        let boundaries = [true];
        let chunks = merge_segments(source, &segments, &boundaries, 100, 0);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].text, "hello world");
        assert_eq!(chunks[0].byte_start, 0);
        assert_eq!(chunks[0].byte_end, 11);
    }

    #[test]
    fn two_segments_same_topic_merged() {
        let source = "first     second";
        let segments = [
            Segment {
                text: &source[0..5],
                byte_start: 0,
            },
            Segment {
                text: &source[10..16],
                byte_start: 10,
            },
        ];
        let boundaries = [true, false];
        let chunks = merge_segments(source, &segments, &boundaries, 200, 0);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].text, "first     second");
        assert_eq!(chunks[0].byte_start, 0);
        assert_eq!(chunks[0].byte_end, 16);
    }

    #[test]
    fn two_segments_different_topics() {
        let source = "topic one           topic two";
        let segments = [
            Segment {
                text: &source[0..9],
                byte_start: 0,
            },
            Segment {
                text: &source[20..29],
                byte_start: 20,
            },
        ];
        let boundaries = [true, true];
        let chunks = merge_segments(source, &segments, &boundaries, 200, 0);
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].text, "topic one");
        assert_eq!(chunks[1].text, "topic two");
    }

    #[test]
    fn oversized_group_falls_back_to_splitter() {
        let long_text = "word ".repeat(100);
        let trimmed = long_text.trim_end();
        let segments = [Segment {
            text: trimmed,
            byte_start: 0,
        }];
        let boundaries = [true];
        let max_chars = 50;
        let chunks = merge_segments(trimmed, &segments, &boundaries, max_chars, 0);
        assert!(chunks.len() > 1, "should split into multiple chunks");
        for chunk in &chunks {
            assert!(
                chunk.text.len() <= max_chars,
                "chunk exceeds budget: {} > {}",
                chunk.text.len(),
                max_chars
            );
        }
    }

    #[test]
    fn overlap_at_topic_boundary() {
        let source = "abcdefghij          klmnop";
        let segments = [
            Segment {
                text: &source[0..10],
                byte_start: 0,
            },
            Segment {
                text: &source[20..26],
                byte_start: 20,
            },
        ];
        let boundaries = [true, true];
        let chunks = merge_segments(source, &segments, &boundaries, 200, 5);
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].text, "abcdefghij");
        assert!(
            chunks[1].text.starts_with("fghij"),
            "expected overlap prefix, got: {:?}",
            chunks[1].text
        );
        assert!(chunks[1].text.contains("klmnop"));
    }

    #[test]
    fn empty_segments() {
        let chunks = merge_segments("", &[], &[], 100, 0);
        assert!(chunks.is_empty());
    }

    #[test]
    fn multiple_groups_with_merge() {
        let source = "a1   a2   b1   b2   c1";
        let segments = [
            Segment {
                text: &source[0..2],
                byte_start: 0,
            },
            Segment {
                text: &source[5..7],
                byte_start: 5,
            },
            Segment {
                text: &source[10..12],
                byte_start: 10,
            },
            Segment {
                text: &source[15..17],
                byte_start: 15,
            },
            Segment {
                text: &source[20..22],
                byte_start: 20,
            },
        ];
        let boundaries = [true, false, true, false, true];
        let chunks = merge_segments(source, &segments, &boundaries, 200, 0);
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].text, "a1   a2");
        assert_eq!(chunks[1].text, "b1   b2");
        assert_eq!(chunks[2].text, "c1");
    }

    #[test]
    fn tail_chars_basic() {
        assert_eq!(tail_chars("hello", 3), "llo");
        assert_eq!(tail_chars("hello", 10), "hello");
        assert_eq!(tail_chars("hello", 0), "");
    }

    #[test]
    fn tail_chars_unicode() {
        let s = "abc🦀🐍";
        assert_eq!(tail_chars(s, 2), "🦀🐍");
        assert_eq!(tail_chars(s, 5), s);
    }

    #[test]
    fn tail_chars_utf8_multibyte() {
        assert_eq!(tail_chars("café", 2), "fé");
    }

    #[test]
    fn tail_chars_zero() {
        assert_eq!(tail_chars("hello", 0), "");
        assert_eq!(tail_chars("café", 0), "");
    }

    #[test]
    fn tail_chars_exceeds_length() {
        assert_eq!(tail_chars("hi", 10), "hi");
        assert_eq!(tail_chars("café", 100), "café");
    }

    #[test]
    fn oversized_group_chunks_report_the_exact_span_of_their_text() {
        let source = format!("preamble  {}", "caf\u{e9} \u{65e5}\u{672c}\u{8a9e} word ".repeat(40));
        let body = source.trim_end();
        let segments = [Segment {
            text: &body[10..],
            byte_start: 10,
        }];
        let chunks = merge_segments(body, &segments, &[true], 30, 0);
        assert!(chunks.len() > 2, "oversized group must be re-split");
        for chunk in &chunks {
            assert_eq!(
                body.get(chunk.byte_start..chunk.byte_end),
                Some(chunk.text.as_str()),
                "offsets must locate the chunk text on char boundaries"
            );
        }
    }

    #[test]
    fn single_segment_exceeds_max_characters() {
        let big = "word ".repeat(200);
        let trimmed = big.trim_end();
        let segments = [Segment {
            text: trimmed,
            byte_start: 0,
        }];
        let boundaries = [true];
        let max = 80;
        let chunks = merge_segments(trimmed, &segments, &boundaries, max, 0);
        assert!(chunks.len() > 1, "oversized single segment must be split");
        for chunk in &chunks {
            assert!(
                chunk.text.len() <= max,
                "chunk exceeds budget: {} > {}",
                chunk.text.len(),
                max
            );
        }
    }

    #[test]
    fn three_groups_middle_oversized() {
        let small_a = "Alpha content";
        let big_b = "word ".repeat(200);
        let big_b_trimmed = big_b.trim_end();
        let small_c = "Gamma content";

        let mut source = String::new();
        source.push_str(small_a);
        source.push_str(&" ".repeat(100 - small_a.len()));
        source.push_str(big_b_trimmed);
        let pad_to = 1200usize.saturating_sub(source.len());
        source.push_str(&" ".repeat(pad_to));
        source.push_str(small_c);

        let segments = [
            Segment {
                text: &source[0..small_a.len()],
                byte_start: 0,
            },
            Segment {
                text: &source[100..100 + big_b_trimmed.len()],
                byte_start: 100,
            },
            Segment {
                text: &source[1200..1200 + small_c.len()],
                byte_start: 1200,
            },
        ];
        let boundaries = [true, true, true];
        let max = 80;
        let chunks = merge_segments(&source, &segments, &boundaries, max, 0);

        assert_eq!(chunks[0].text, small_a);
        assert_eq!(chunks.last().unwrap().text, small_c);
        let middle_chunks: Vec<_> = chunks[1..chunks.len() - 1].to_vec();
        assert!(
            middle_chunks.len() > 1,
            "oversized middle group should produce multiple chunks"
        );
        for mc in &middle_chunks {
            assert!(mc.text.len() <= max, "middle chunk exceeds budget");
        }
    }

    #[test]
    fn merge_with_overlap_and_oversized_group() {
        let small_a = "alpha text here";
        let big_b = "word ".repeat(100);
        let big_b_trimmed = big_b.trim_end();
        let small_c = "gamma text";

        let mut source = String::new();
        source.push_str(small_a);
        source.push_str(&" ".repeat(50 - small_a.len()));
        source.push_str(big_b_trimmed);
        let pad_to = 600usize.saturating_sub(source.len());
        source.push_str(&" ".repeat(pad_to));
        source.push_str(small_c);

        let segments = [
            Segment {
                text: &source[0..small_a.len()],
                byte_start: 0,
            },
            Segment {
                text: &source[50..50 + big_b_trimmed.len()],
                byte_start: 50,
            },
            Segment {
                text: &source[600..600 + small_c.len()],
                byte_start: 600,
            },
        ];
        let boundaries = [true, true, true];
        let max = 60;
        let overlap = 5;
        let chunks = merge_segments(&source, &segments, &boundaries, max, overlap);

        assert_eq!(chunks[0].text, small_a);

        let last = chunks.last().unwrap();
        assert!(last.text.contains(small_c), "last chunk should contain its own text");

        for chunk in &chunks[1..chunks.len() - 1] {
            assert!(
                chunk.text.len() <= max,
                "middle chunk exceeds budget: {} > {}",
                chunk.text.len(),
                max
            );
        }
    }

    #[test]
    fn tail_chars_cjk_characters() {
        let s = "hello\u{4e16}\u{754c}";
        assert_eq!(tail_chars(s, 2), "\u{4e16}\u{754c}");
        assert_eq!(tail_chars(s, 7), s);
    }

    #[test]
    fn tail_chars_cafe_accent() {
        let s = "cafe\u{0301}";
        assert_eq!(tail_chars(s, 3), "fe\u{0301}");
    }
}
