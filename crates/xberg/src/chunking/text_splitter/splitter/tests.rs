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

fn legacy_level_ranges(ranges: &SemanticSplitRanges<usize>, offset: usize, level: usize) -> Vec<(usize, Range<usize>)> {
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
