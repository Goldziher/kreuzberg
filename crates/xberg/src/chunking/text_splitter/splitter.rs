//! Vendored from text-splitter v0.30.1 (MIT, © 2023 Benjamin Brandt). See ATTRIBUTIONS.md.

use std::{cmp::Ordering, collections::BTreeMap, fmt, iter::once, ops::Range};

use either::Either;
use itertools::Itertools;
use strum::IntoEnumIterator;

use self::fallback::FallbackLevel;
use super::{ChunkCapacity, ChunkConfig, ChunkSizer, chunk_size::MemoizedChunkSizer, trim::Trim};

mod fallback;
mod markdown;
mod text;

pub(crate) use markdown::MarkdownSplitter;
pub(crate) use text::TextSplitter;

/// Shared interface for splitters that can generate chunks of text based on the
/// associated semantic level.
trait Splitter<Sizer>
where
    Sizer: ChunkSizer,
{
    type Level: SemanticLevel;

    /// Trimming behavior to use when trimming chunks
    const TRIM: Trim = Trim::All;

    /// Retrieve the splitter chunk configuration
    fn chunk_config(&self) -> &ChunkConfig<Sizer>;

    /// Generate a list of offsets for each semantic level within the text.
    fn parse(&self, text: &str) -> Vec<(Self::Level, Range<usize>)>;

    /// Returns an iterator over chunks of the text and their byte offsets.
    /// Each chunk will be up to the max size of the `ChunkConfig`.
    fn chunk_indices<'splitter, 'text: 'splitter>(
        &'splitter self,
        text: &'text str,
    ) -> impl Iterator<Item = (usize, &'text str)> + 'splitter
    where
        Sizer: 'splitter,
    {
        TextChunks::<Sizer, Self::Level>::new(self.chunk_config(), text, self.parse(text), Self::TRIM)
    }

    /// Generate a list of chunks from a given text.
    /// Each chunk will be up to the max size of the `ChunkConfig`.
    fn chunks<'splitter, 'text: 'splitter>(
        &'splitter self,
        text: &'text str,
    ) -> impl Iterator<Item = &'text str> + 'splitter
    where
        Sizer: 'splitter,
    {
        self.chunk_indices(text).map(|(_, t)| t)
    }
}

/// Custom-defined levels of semantic splitting for custom document types.
trait SemanticLevel: Copy + fmt::Debug + Ord + PartialOrd + 'static {
    /// Given a level, split the text into sections based on the level.
    /// Level ranges are also provided of items that are equal to or greater than the current level.
    /// Default implementation assumes that all level ranges should be treated
    /// as their own item.
    fn sections(
        text: &str,
        level_ranges: impl Iterator<Item = (Self, Range<usize>)>,
    ) -> impl Iterator<Item = (usize, &str)> {
        let mut cursor = 0;
        let mut final_match = false;
        level_ranges
            .batching(move |it| {
                loop {
                    match it.next() {
                        None if final_match => return None,
                        None => {
                            final_match = true;
                            return text.get(cursor..).map(|t| Either::Left(once((cursor, t))));
                        }
                        Some((_, range)) => {
                            if range.start < cursor {
                                continue;
                            }
                            let offset = cursor;
                            let prev_section = text.get(offset..range.start).expect("invalid character sequence");
                            let separator = text.get(range.start..range.end).expect("invalid character sequence");
                            cursor = range.end;
                            return Some(Either::Right(
                                [(offset, prev_section), (range.start, separator)].into_iter(),
                            ));
                        }
                    }
                }
            })
            .flatten()
            .filter(|(_, s)| !s.is_empty())
    }
}

/// Captures information about document structure for a given text, and their
/// various semantic levels
#[derive(Debug)]
struct SemanticSplitRanges<Level>
where
    Level: SemanticLevel,
{
    /// Current cursor in the ranges list, so that we can skip over items we've
    /// already processed.
    cursor: usize,
    /// Indexes each level's occurrences so future chunks never rescan the remaining tail. ~keep
    level_indices: BTreeMap<Level, Vec<usize>>,
    /// Supports successor queries by minimum level without visiting lower-level ranges. ~keep
    level_tree: LevelRangeTree<Level>,
    /// Range of each semantic item and its precalculated semantic level
    ranges: Vec<(Level, Range<usize>)>,
}

#[derive(Debug)]
struct LevelRangeTree<Level>
where
    Level: SemanticLevel,
{
    leaf_count: usize,
    levels: Vec<Option<Level>>,
    range_count: usize,
}

impl<Level> LevelRangeTree<Level>
where
    Level: SemanticLevel,
{
    fn new(ranges: &[(Level, Range<usize>)]) -> Self {
        let leaf_count = ranges.len().next_power_of_two().max(1);
        let mut levels = vec![None; leaf_count * 2];
        for (index, (level, _)) in ranges.iter().enumerate() {
            levels[leaf_count + index] = Some(*level);
        }
        for index in (1..leaf_count).rev() {
            levels[index] = levels[index * 2].max(levels[index * 2 + 1]);
        }
        Self {
            leaf_count,
            levels,
            range_count: ranges.len(),
        }
    }

    fn first_at_or_above(&self, start: usize, minimum: Level) -> Option<usize> {
        if start >= self.range_count {
            return None;
        }
        self.first_in_node(1, 0, self.leaf_count, start, minimum)
            .filter(|index| *index < self.range_count)
    }

    fn first_in_node(
        &self,
        node: usize,
        node_start: usize,
        node_end: usize,
        start: usize,
        minimum: Level,
    ) -> Option<usize> {
        if node_end <= start || self.levels[node].is_none_or(|maximum| maximum < minimum) {
            return None;
        }
        if node_end - node_start == 1 {
            return Some(node_start);
        }

        let midpoint = node_start + (node_end - node_start) / 2;
        self.first_in_node(node * 2, node_start, midpoint, start, minimum)
            .or_else(|| self.first_in_node(node * 2 + 1, midpoint, node_end, start, minimum))
    }
}

impl<Level> SemanticSplitRanges<Level>
where
    Level: SemanticLevel,
{
    fn new(mut ranges: Vec<(Level, Range<usize>)>) -> Self {
        ranges.sort_unstable_by(|(_, a), (_, b)| a.start.cmp(&b.start).then_with(|| b.end.cmp(&a.end)));
        let mut level_indices = BTreeMap::<Level, Vec<usize>>::new();
        for (index, (level, _)) in ranges.iter().enumerate() {
            level_indices.entry(*level).or_default().push(index);
        }
        let level_tree = LevelRangeTree::new(&ranges);
        Self {
            cursor: 0,
            level_indices,
            level_tree,
            ranges,
        }
    }

    fn index_after_offset(&self, offset: usize) -> usize {
        self.cursor + self.ranges[self.cursor..].partition_point(|(_, range)| range.start < offset)
    }

    /// Retrieve ranges for all sections of a given level after an offset
    #[cfg(test)]
    fn ranges_after_offset(&self, offset: usize) -> impl Iterator<Item = (Level, Range<usize>)> + '_ {
        self.ranges[self.index_after_offset(offset)..]
            .iter()
            .map(|(l, r)| (*l, r.start..r.end))
    }
    /// Retrieve ranges for all sections of a given level after an offset
    fn level_ranges_after_offset(
        &self,
        offset: usize,
        level: Level,
    ) -> impl Iterator<Item = (Level, Range<usize>)> + '_ {
        let start_index = self.index_after_offset(offset);
        let first_item = self.level_indices.get(&level).and_then(|indices| {
            let position = indices.partition_point(|index| *index < start_index);
            let mut index = *indices.get(position)?;
            if index == start_index {
                let start = self.ranges[index].1.start;
                while self
                    .ranges
                    .get(index + 1)
                    .is_some_and(|(next_level, next_range)| *next_level == level && next_range.start == start)
                {
                    index += 1;
                }
            }
            let (first_level, first_range) = &self.ranges[index];
            Some((*first_level, first_range.clone()))
        });

        let mut next_index = start_index;
        std::iter::from_fn(move || {
            let index = self.level_tree.first_at_or_above(next_index, level)?;
            next_index = index + 1;
            let (item_level, range) = &self.ranges[index];
            Some((*item_level, range.clone()))
        })
        .skip_while(move |(l, r)| {
            first_item.as_ref().is_some_and(|(_, fir)| {
                (l > &level && r.contains(&fir.start)) || (l == &level && r.start == fir.start && r.end > fir.end)
            })
        })
    }

    /// Return a unique, sorted list of all line break levels present before the next max level, added
    /// to all of the base semantic levels, in order from smallest to largest
    fn levels_in_remaining_text(&self, offset: usize) -> impl Iterator<Item = Level> + '_ {
        let start_index = self.index_after_offset(offset);
        self.level_indices
            .iter()
            .filter(move |(_, indices)| indices.last().is_some_and(|index| *index >= start_index))
            .map(|(level, _)| *level)
    }

    /// Split a given text into iterator over each semantic chunk
    fn semantic_chunks<'splitter, 'text: 'splitter>(
        &'splitter self,
        offset: usize,
        text: &'text str,
        semantic_level: Level,
    ) -> impl Iterator<Item = (usize, &'text str)> + 'splitter {
        Level::sections(
            text,
            self.level_ranges_after_offset(offset, semantic_level)
                .map(move |(l, sep)| (l, sep.start - offset..sep.end - offset)),
        )
        .map(move |(i, str)| (offset + i, str))
    }

    /// Clear out ranges we have moved past so future iterations are faster
    fn update_cursor(&mut self, cursor: usize) {
        self.cursor = self.index_after_offset(cursor);
    }
}

/// Returns chunks of text with their byte offsets as an iterator.
#[derive(Debug)]
struct TextChunks<'text, 'sizer, Sizer, Level>
where
    Sizer: ChunkSizer,
    Level: SemanticLevel,
{
    /// Overall capacity of the chunk
    capacity: ChunkCapacity,
    /// How to validate chunk sizes
    chunk_sizer: MemoizedChunkSizer<'sizer, Sizer>,
    /// Average number of sections in a chunk for each level
    chunk_stats: ChunkStats,
    /// Current byte offset in the `text`
    cursor: usize,
    /// Reusable container for next sections to avoid extra allocations
    next_sections: Vec<(usize, &'text str)>,
    /// Overlap capacity
    overlap: ChunkCapacity,
    /// Previous item's end byte offset
    prev_item_end: usize,
    /// Splitter used for determining semantic levels.
    semantic_split: SemanticSplitRanges<Level>,
    /// Original text to iterate over and generate chunks from
    text: &'text str,
    /// The trimming method to apply
    trim: Trim,
}

impl<'sizer, 'text: 'sizer, Sizer, Level> TextChunks<'text, 'sizer, Sizer, Level>
where
    Sizer: ChunkSizer,
    Level: SemanticLevel,
{
    /// Generate new [`TextChunks`] iterator for a given text.
    /// Starts with an offset of 0
    fn new(
        chunk_config: &'sizer ChunkConfig<Sizer>,
        text: &'text str,
        offsets: Vec<(Level, Range<usize>)>,
        trim: Trim,
    ) -> Self {
        let ChunkConfig {
            capacity,
            overlap,
            sizer,
            trim: trim_enabled,
        } = chunk_config;
        Self {
            capacity: *capacity,
            chunk_sizer: MemoizedChunkSizer::new(sizer),
            chunk_stats: ChunkStats::new(),
            cursor: 0,
            next_sections: Vec::new(),
            overlap: (*overlap).into(),
            prev_item_end: 0,
            semantic_split: SemanticSplitRanges::new(offsets),
            text,
            trim: if *trim_enabled { trim } else { Trim::None },
        }
    }

    /// Generate the next chunk, applying trimming settings.
    /// Returns final byte offset and str.
    /// Will return `None` if given an invalid range.
    fn next_chunk(&mut self) -> Option<(usize, &'text str)> {
        self.semantic_split.update_cursor(self.cursor);
        let low = self.update_next_sections(false);
        let (mut start, mut end) = self.binary_search_next_chunk(low)?;
        if end < self.text.len() && is_mark_only(self.text.get(start..end)?) {
            let low = self.update_next_sections(true);
            (start, end) = self.binary_search_next_chunk(low)?;
        }
        let chunk = self.text.get(start..end)?;
        self.chunk_stats.update_max_chunk_size(end - start);

        self.chunk_sizer.clear_cache();
        self.update_cursor(end);

        Some(self.trim.trim(start, chunk))
    }

    /// Use binary search to find the next chunk that fits within the chunk size
    fn binary_search_next_chunk(&mut self, mut low: usize) -> Option<(usize, usize)> {
        let start = self.cursor;
        let mut end = self.cursor;
        let mut equals_found = false;
        let mut high = self.next_sections.len().saturating_sub(1);
        let mut successful_index = None;
        let mut successful_chunk_size = None;

        while low <= high {
            let mid = low + (high - low) / 2;
            let (offset, str) = self.next_sections[mid];
            let text_end = offset + str.len();
            let chunk = self.text.get(start..text_end)?;
            let chunk_size = self.chunk_sizer.chunk_size(start, chunk, self.trim);
            let fits = self.capacity.fits(chunk_size);

            match fits {
                Ordering::Less => {
                    if text_end > end {
                        end = text_end;
                        successful_index = Some(mid);
                        successful_chunk_size = Some(chunk_size);
                    }
                }
                Ordering::Equal => {
                    if text_end < end || !equals_found {
                        end = text_end;
                        successful_index = Some(mid);
                        successful_chunk_size = Some(chunk_size);
                    }
                    equals_found = true;
                }
                Ordering::Greater => {
                    if mid == 0 && start == end {
                        end = text_end;
                        successful_index = Some(mid);
                        successful_chunk_size = Some(chunk_size);
                    }
                }
            }

            if fits.is_lt() {
                low = mid + 1;
            } else if mid > 0 {
                high = mid - 1;
            } else {
                break;
            }
        }

        if let (Some(successful_index), Some(chunk_size)) = (successful_index, successful_chunk_size) {
            let mut range = successful_index..self.next_sections.len();
            range.next();

            for index in range {
                let (offset, str) = self.next_sections[index];
                let text_end = offset + str.len();
                let chunk = self.text.get(start..text_end)?;
                let size = self.chunk_sizer.chunk_size(start, chunk, self.trim);
                if size <= chunk_size {
                    if text_end > end {
                        end = text_end;
                    }
                } else {
                    break;
                }
            }
        }

        Some((start, end))
    }

    /// Use binary search to find the sections that fit within the overlap size.
    /// If no overlap deisired, return end.
    fn update_cursor(&mut self, end: usize) {
        if self.overlap.max == 0 {
            self.cursor = end;
            return;
        }

        let mut start = end;
        let mut low = 0;
        let mut high = match self
            .next_sections
            .binary_search_by_key(&end, |(offset, str)| offset + str.len())
        {
            Ok(i) | Err(i) => i,
        };

        while low <= high {
            let mid = low + (high - low) / 2;
            let (offset, _) = self.next_sections[mid];
            let chunk_size =
                self.chunk_sizer
                    .chunk_size(offset, self.text.get(offset..end).expect("Invalid range"), self.trim);
            let fits = self.overlap.fits(chunk_size);

            if fits.is_le() && offset < start && offset > self.cursor {
                start = offset;
            }

            if fits.is_lt() && mid > 0 {
                high = mid - 1;
            } else {
                low = mid + 1;
            }
        }

        self.cursor = start;
    }

    /// The deepest semantic level whose first section still fits, with the offset cap it implies.
    fn semantic_level_and_offset(
        &mut self,
        remaining_text: &'text str,
        avoid_dangling: bool,
    ) -> (Option<Level>, Option<usize>) {
        self.chunk_sizer.find_correct_level(
            self.cursor,
            &self.capacity,
            self.semantic_split
                .levels_in_remaining_text(self.cursor)
                .filter_map(|level| {
                    self.semantic_split
                        .semantic_chunks(self.cursor, remaining_text, level)
                        .next()
                        .map(|(_, str)| (level, str))
                })
                .filter(|&(_, str)| !(avoid_dangling && is_mark_only(str))),
            self.trim,
        )
    }

    /// The prefix of `remaining_text` the fallback levels need to look at.
    ///
    /// Unicode sentence and dictionary word segmentation scan the whole remaining text to find
    /// the first segment, which made every chunk of a document with no newline levels cost
    /// O(remaining) and the whole split quadratic. A chunk can never extend past the first
    /// prefix that already exceeds the capacity, so the levels are computed on a prefix that
    /// does, grown by doubling.
    ///
    /// The prefix ends where segmentation is independent of the text that was cut off. Sentence
    /// rules can look arbitrarily far ahead only through an `ATerm` (`.`) that is not yet
    /// followed by a letter, newline or other sentence terminator, so the prefix stops before
    /// such a trailing `.`; other rules look a few characters ahead, covered by a fixed margin.
    /// It relies on `size` never shrinking when text is appended, which holds for every sizer
    /// the splitter ships.
    fn fallback_window(&mut self, remaining_text: &'text str) -> &'text str {
        let total = remaining_text.len();
        let mut len = self.capacity.max.saturating_mul(4).max(MIN_WINDOW_BYTES);
        loop {
            if len >= total {
                return remaining_text;
            }
            let end = stable_prefix_end(remaining_text, len);
            let window = &remaining_text[..end];
            if end > 0 {
                let size = self.chunk_sizer.chunk_size(self.cursor, window, self.trim);
                if self.capacity.fits(size).is_gt() {
                    return window;
                }
            }
            len = len.saturating_mul(2);
        }
    }

    /// The fallback level to split on when no semantic level fits, with `max_offset` narrowed to it.
    fn fallback_level_and_offset(
        &mut self,
        window: &'text str,
        max_offset: Option<usize>,
        avoid_dangling: bool,
    ) -> (FallbackLevel, Option<usize>) {
        let (semantic_level, fallback_max_offset) = self.chunk_sizer.find_correct_level(
            self.cursor,
            &self.capacity,
            FallbackLevel::iter()
                .filter_map(|level| level.sections(window).next().map(|(_, str)| (level, str)))
                .filter(|&(level, str)| !(avoid_dangling && level == FallbackLevel::Sentence && is_mark_only(str))),
            self.trim,
        );

        let max_offset = match (fallback_max_offset, max_offset) {
            (Some(fallback), Some(max)) => Some(fallback.min(max)),
            (fallback, max) => fallback.or(max),
        };

        (semantic_level.unwrap_or(FallbackLevel::Char), max_offset)
    }

    /// Find the ideal next sections, breaking it up until we find the largest chunk.
    /// Increasing length of chunk until we find biggest size to minimize validation time
    /// on huge chunks
    fn update_next_sections(&mut self, avoid_dangling: bool) -> usize {
        self.next_sections.clear();

        let remaining_text = self.text.get(self.cursor..).unwrap();

        let (semantic_level, mut max_offset) = self.semantic_level_and_offset(remaining_text, avoid_dangling);

        let sections = if let Some(semantic_level) = semantic_level {
            Either::Left(
                self.semantic_split
                    .semantic_chunks(self.cursor, remaining_text, semantic_level),
            )
        } else {
            let window = self.fallback_window(remaining_text);
            let (fallback_level, merged_max_offset) =
                self.fallback_level_and_offset(window, max_offset, avoid_dangling);
            max_offset = merged_max_offset;

            Either::Right(
                fallback_level
                    .sections(window)
                    .map(|(offset, text)| (self.cursor + offset, text)),
            )
        };

        let mut sections = sections
            .take_while(move |(offset, _)| max_offset.is_none_or(|max| *offset <= max))
            .filter(|(_, str)| !str.is_empty());

        let mut low = 0;
        let mut prev_equals: Option<usize> = None;
        let max = self.capacity.max;
        let mut target_offset = self.chunk_stats.max_chunk_size.unwrap_or(max);

        loop {
            let offset_limit = self.cursor.saturating_add(target_offset);
            let added = push_sections_until(&mut self.next_sections, &mut sections, offset_limit);
            let new_num = self.next_sections.len();
            if added == 0 {
                break;
            }

            if let Some(&(offset, str)) = self.next_sections.last() {
                let text_end = offset + str.len();
                if (text_end - self.cursor) < target_offset {
                    break;
                }
                let chunk_size = self.chunk_sizer.chunk_size(
                    offset,
                    self.text.get(self.cursor..text_end).expect("Invalid range"),
                    self.trim,
                );
                let fits = self.capacity.fits(chunk_size);

                if fits.is_le() {
                    let final_offset = offset + str.len() - self.cursor;
                    target_offset = grown_target_offset(final_offset, chunk_size, max);
                }

                match fits {
                    Ordering::Less => {
                        low = new_num.saturating_sub(1);
                    }
                    Ordering::Equal => {
                        if let Some(prev) = prev_equals
                            && prev < chunk_size
                        {
                            break;
                        }
                        prev_equals = Some(chunk_size);
                    }
                    Ordering::Greater => {
                        break;
                    }
                }
            }
        }

        low
    }
}

/// Whether `text` holds only punctuation, markup marks and whitespace, with no letter or digit.
///
/// A cut that lands just before a sentence's closing mark, or a list bullet, heading marker or
/// table pipe that is followed by a unit too long to join it, can leave a chunk that carries no
/// content at all. Such a chunk is redone without anchoring to the level that produced it, which
/// fills it from finer units instead.
fn is_mark_only(text: &str) -> bool {
    !text.trim().is_empty()
        && text.chars().all(|c| {
            c.is_whitespace()
                || c.is_ascii_punctuation()
                || matches!(
                    c,
                    '\u{2026}'
                        | '\u{2025}'
                        | '\u{201d}'
                        | '\u{2019}'
                        | '\u{bb}'
                        | '\u{3001}'
                        | '\u{3002}'
                        | '\u{ff01}'
                        | '\u{ff09}'
                        | '\u{ff0c}'
                        | '\u{ff1a}'
                        | '\u{ff1b}'
                        | '\u{ff1f}'
                        | '\u{300d}'
                        | '\u{300f}'
                        | '\u{3011}'
                        | '\u{61f}'
                        | '\u{60c}'
                        | '\u{61b}'
                        | '\u{6d4}'
                        | '\u{964}'
                        | '\u{965}'
                )
        })
}

/// Smallest fallback lookahead window, so tiny capacities still see a few words.
const MIN_WINDOW_BYTES: usize = 256;

/// Bytes kept clear of the window end for rules that look a few characters ahead.
const LOOKAHEAD_MARGIN_BYTES: usize = 64;

/// End of the longest prefix of `text[..len]` whose fallback segmentation matches the full text.
fn stable_prefix_end(text: &str, len: usize) -> usize {
    let cut = text.floor_char_boundary(len.saturating_sub(LOOKAHEAD_MARGIN_BYTES));
    let head = &text[..cut];
    let last_terminator = head.char_indices().rev().find(|&(_, c)| {
        c.is_alphabetic() || matches!(c, '.' | '!' | '?' | '\n' | '\r' | '\u{85}' | '\u{2028}' | '\u{2029}')
    });
    match last_terminator {
        Some((position, '.')) => position,
        _ => cut,
    }
}

/// Drain `sections` into `next_sections` until one reaches past `offset_limit`, returning how many were added.
fn push_sections_until<'text>(
    next_sections: &mut Vec<(usize, &'text str)>,
    sections: &mut impl Iterator<Item = (usize, &'text str)>,
    offset_limit: usize,
) -> usize {
    let prev_num = next_sections.len();
    for (offset, str) in sections.by_ref() {
        next_sections.push((offset, str));
        if offset + str.len() > offset_limit {
            break;
        }
    }
    next_sections.len() - prev_num
}

fn grown_target_offset(final_offset: usize, chunk_size: usize, max: usize) -> usize {
    let size = chunk_size.max(1);
    let diff = (max - size).max(1);
    let avg_size = final_offset.div_ceil(size);
    final_offset
        .saturating_add(diff.saturating_mul(avg_size))
        .saturating_add(final_offset.div_ceil(10))
}

impl<'sizer, 'text: 'sizer, Sizer, Level> Iterator for TextChunks<'text, 'sizer, Sizer, Level>
where
    Sizer: ChunkSizer,
    Level: SemanticLevel,
{
    type Item = (usize, &'text str);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.cursor >= self.text.len() {
                return None;
            }

            match self.next_chunk()? {
                (_, "") => {}
                c => {
                    let item_end = c.0 + c.1.len();
                    if item_end <= self.prev_item_end {
                        continue;
                    }
                    self.prev_item_end = item_end;
                    return Some(c);
                }
            }
        }
    }
}

/// Keeps track of the average size of chunks as we go
#[derive(Debug, Default)]
struct ChunkStats {
    /// The size of the biggest chunk we've seen, if we have seen at least one
    max_chunk_size: Option<usize>,
}

impl ChunkStats {
    fn new() -> Self {
        Self::default()
    }

    /// Update statistics after the chunk has been produced
    fn update_max_chunk_size(&mut self, size: usize) {
        self.max_chunk_size = self.max_chunk_size.map(|s| s.max(size)).or(Some(size));
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, cmp::Ordering};

    use super::*;

    thread_local! {
        static LEVEL_COMPARISONS: Cell<usize> = const { Cell::new(0) };
    }

    #[derive(Clone, Copy, Debug)]
    struct CountingLevel(u8);

    impl CountingLevel {
        fn reset_comparisons() {
            LEVEL_COMPARISONS.set(0);
        }

        fn comparisons() -> usize {
            LEVEL_COMPARISONS.get()
        }

        fn record_comparison() {
            LEVEL_COMPARISONS.set(LEVEL_COMPARISONS.get() + 1);
        }
    }

    impl PartialEq for CountingLevel {
        fn eq(&self, other: &Self) -> bool {
            Self::record_comparison();
            self.0 == other.0
        }
    }

    impl Eq for CountingLevel {}

    impl PartialOrd for CountingLevel {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            Some(self.cmp(other))
        }
    }

    impl Ord for CountingLevel {
        fn cmp(&self, other: &Self) -> Ordering {
            Self::record_comparison();
            self.0.cmp(&other.0)
        }
    }

    impl SemanticLevel for CountingLevel {}

    fn legacy_level_ranges(
        ranges: &SemanticSplitRanges<usize>,
        offset: usize,
        level: usize,
    ) -> Vec<(usize, Range<usize>)> {
        let remaining = ranges.ranges_after_offset(offset).collect::<Vec<_>>();
        let first_item = remaining
            .iter()
            .position(|(item_level, _)| *item_level == level)
            .map(|position| {
                let mut first = remaining[position].clone();
                if position == 0 {
                    for candidate in &remaining[position + 1..] {
                        if candidate.0 != first.0 || candidate.1.start != first.1.start {
                            break;
                        }
                        first = candidate.clone();
                    }
                }
                first
            });
        remaining
            .into_iter()
            .filter(|(item_level, _)| *item_level >= level)
            .skip_while(|(item_level, range)| {
                first_item.as_ref().is_some_and(|(_, first_range)| {
                    (*item_level > level && range.contains(&first_range.start))
                        || (*item_level == level && range.start == first_range.start && range.end > first_range.end)
                })
            })
            .collect()
    }

    #[test]
    fn chunk_stats_empty() {
        let stats = ChunkStats::new();
        assert_eq!(stats.max_chunk_size, None);
    }

    #[test]
    fn chunk_stats_one() {
        let mut stats = ChunkStats::new();
        stats.update_max_chunk_size(10);
        assert_eq!(stats.max_chunk_size, Some(10));
    }

    #[test]
    fn chunk_stats_multiple() {
        let mut stats = ChunkStats::new();
        stats.update_max_chunk_size(10);
        stats.update_max_chunk_size(20);
        stats.update_max_chunk_size(30);
        assert_eq!(stats.max_chunk_size, Some(30));
    }

    impl SemanticLevel for usize {}

    #[test]
    fn semantic_ranges_are_sorted() {
        let ranges = SemanticSplitRanges::new(vec![(0, 0..1), (1, 0..2), (0, 1..2), (2, 0..4)]);

        assert_eq!(ranges.ranges, vec![(2, 0..4), (1, 0..2), (0, 0..1), (0, 1..2)]);
    }

    #[test]
    fn semantic_ranges_skip_previous_ranges() {
        let mut ranges = SemanticSplitRanges::new(vec![(0, 0..1), (1, 0..2), (0, 1..2), (2, 0..4)]);

        ranges.update_cursor(1);

        assert_eq!(ranges.ranges_after_offset(0).collect::<Vec<_>>(), vec![(0, 1..2)]);
    }

    #[test]
    fn remaining_levels_do_not_rescan_all_future_ranges() {
        let input = (0..8_192)
            .map(|index| (CountingLevel((index % 2) as u8), index..index + 1))
            .collect();
        let mut ranges = SemanticSplitRanges::new(input);
        CountingLevel::reset_comparisons();

        for offset in (0..8_192).step_by(128) {
            ranges.update_cursor(offset);
            assert_eq!(ranges.levels_in_remaining_text(offset).count(), 2);
        }

        assert!(
            CountingLevel::comparisons() < 1_000,
            "remaining-level lookup performed {} level comparisons",
            CountingLevel::comparisons()
        );
    }

    #[test]
    fn distant_higher_level_lookup_does_not_scan_lower_levels() {
        let mut input = (0..8_191)
            .map(|index| (CountingLevel(0), index..index + 1))
            .collect::<Vec<_>>();
        input.push((CountingLevel(1), 8_191..8_192));
        let ranges = SemanticSplitRanges::new(input);
        CountingLevel::reset_comparisons();

        assert_eq!(
            ranges
                .level_ranges_after_offset(0, CountingLevel(1))
                .next()
                .map(|(_, range)| range),
            Some(8_191..8_192)
        );
        assert!(
            CountingLevel::comparisons() < 256,
            "higher-level lookup performed {} level comparisons",
            CountingLevel::comparisons()
        );
    }

    #[test]
    fn indexed_level_ranges_match_legacy_nested_range_semantics() {
        let input = vec![
            (3, 0..20),
            (2, 0..15),
            (1, 0..10),
            (1, 0..8),
            (0, 0..4),
            (0, 4..8),
            (2, 8..20),
            (1, 8..16),
            (0, 8..12),
            (0, 12..16),
            (3, 20..30),
            (0, 20..25),
        ];

        for offset in [0, 4, 8, 12, 20] {
            let mut ranges = SemanticSplitRanges::new(input.clone());
            ranges.update_cursor(offset);
            for level in 0..=3 {
                let expected = legacy_level_ranges(&ranges, offset, level);

                assert_eq!(
                    ranges.level_ranges_after_offset(offset, level).collect::<Vec<_>>(),
                    expected,
                    "offset={offset}, level={level}"
                );
            }
        }
    }
}
