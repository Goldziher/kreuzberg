//! Page boundary handling and page range calculation for chunked text.
//!
//! This module provides functions to track which pages text chunks span,
//! enabling accurate page-level metadata for document processing.

use crate::error::{Result, XbergError};
use crate::types::{PageBoundary, PageSpan};

/// Validates the consistency and correctness of page boundaries.
///
/// # Validation Rules
///
/// 1. Boundaries must be sorted by byte_start (monotonically increasing)
/// 2. Boundaries must not overlap (byte_end[i] <= byte_start[i+1])
/// 3. Each boundary must have byte_start < byte_end
///
/// # Arguments
///
/// * `boundaries` - Page boundary markers to validate
///
/// # Returns
///
/// Returns `Ok(())` if all boundaries are valid.
/// Returns `XbergError::Validation` if any boundary is invalid.
pub(crate) fn validate_page_boundaries(boundaries: &[PageBoundary]) -> Result<()> {
    if boundaries.is_empty() {
        return Ok(());
    }

    for (idx, boundary) in boundaries.iter().enumerate() {
        if boundary.byte_start > boundary.byte_end {
            return Err(XbergError::validation(format!(
                "Invalid boundary range at index {}: byte_start ({}) must be <= byte_end ({})",
                idx, boundary.byte_start, boundary.byte_end
            )));
        }
    }

    for i in 0..boundaries.len() - 1 {
        let current = &boundaries[i];
        let next = &boundaries[i + 1];

        if current.byte_start > next.byte_start {
            return Err(XbergError::validation(format!(
                "Page boundaries not sorted: boundary at index {} (byte_start={}) comes after boundary at index {} (byte_start={})",
                i,
                current.byte_start,
                i + 1,
                next.byte_start
            )));
        }

        if current.byte_end > next.byte_start {
            return Err(XbergError::validation(format!(
                "Overlapping page boundaries: boundary {} ends at {} but boundary {} starts at {}",
                i,
                current.byte_end,
                i + 1,
                next.byte_start
            )));
        }
    }

    Ok(())
}

/// Validated page boundaries with an indexed overlap lookup.
///
/// Boundaries are validated once, at construction, instead of on every query. Validation
/// guarantees both `byte_start` and `byte_end` are non-decreasing, so the boundaries
/// overlapping a byte range are a contiguous run that a binary search locates in `O(log pages)`.
/// Resolving every chunk of a large paginated document is therefore `O(chunks * log pages)`
/// rather than `O(chunks * pages)`.
pub(crate) struct PageIndex<'a> {
    boundaries: &'a [PageBoundary],
}

impl<'a> PageIndex<'a> {
    /// Validates `boundaries` once.
    ///
    /// # Errors
    ///
    /// Returns `XbergError::Validation` if the boundaries are unsorted, overlapping or inverted.
    pub(crate) fn new(boundaries: &'a [PageBoundary]) -> Result<Self> {
        validate_page_boundaries(boundaries)?;
        Ok(Self { boundaries })
    }

    /// The boundaries overlapping `[byte_start, byte_end)`, in page order. A boundary overlaps
    /// when `byte_start < boundary.byte_end && byte_end > boundary.byte_start`.
    fn overlapping(&self, byte_start: usize, byte_end: usize) -> impl Iterator<Item = &'a PageBoundary> {
        let boundaries = self.boundaries;
        let first = boundaries.partition_point(|b| b.byte_end <= byte_start);
        boundaries[first..].iter().take_while(move |b| b.byte_start < byte_end)
    }

    /// `(first_page, last_page)` of the pages a byte range spans, 1-indexed.
    ///
    /// `(None, None)` when there are no boundaries or the range overlaps no page.
    pub(crate) fn range(&self, byte_start: usize, byte_end: usize) -> (Option<u32>, Option<u32>) {
        let mut overlapping = self.overlapping(byte_start, byte_end);
        let first = overlapping.next().map(|b| b.page_number);
        let last = overlapping.last().map(|b| b.page_number).or(first);
        (first, last)
    }

    /// The ordered [`PageSpan`]s of the pages a byte range spans, each with `bbox: None`.
    ///
    /// Uses the same overlap test as [`Self::range`], so the first and last spans' pages equal
    /// `first_page`/`last_page`. Bounding boxes are filled in by a later pass (see
    /// `chunking::page_spans::populate_page_span_bboxes`) once the document's structured node
    /// tree is available.
    pub(crate) fn spans(&self, byte_start: usize, byte_end: usize) -> Vec<PageSpan> {
        self.overlapping(byte_start, byte_end)
            .map(|b| PageSpan {
                page: b.page_number,
                bbox: None,
            })
            .collect()
    }

    /// Byte offset where the page containing `offset` begins.
    ///
    /// Falls back to the closest preceding boundary's start when `offset` falls strictly
    /// within no `[byte_start, byte_end)` range (a zero-width boundary for a blank page, a gap
    /// between pages, or an offset at content end). `None` if no boundary starts at or before
    /// `offset`.
    pub(crate) fn page_start(&self, offset: usize) -> Option<usize> {
        let boundaries = self.boundaries;
        let containing = boundaries.partition_point(|b| b.byte_end <= offset);
        if let Some(b) = boundaries.get(containing)
            && b.byte_start <= offset
        {
            return Some(b.byte_start);
        }
        let preceding = boundaries.partition_point(|b| b.byte_start <= offset);
        preceding.checked_sub(1).map(|i| boundaries[i].byte_start)
    }
}

/// Calculate which pages a byte range spans.
///
/// Returns `(first_page, last_page)`, 1-indexed, or `(None, None)` if `boundaries` is empty or
/// the range overlaps no page. Callers resolving many ranges against one boundary set build a
/// [`PageIndex`] once instead.
///
/// # Errors
///
/// Returns `XbergError::Validation` if boundaries are invalid.
#[cfg(test)]
pub(crate) fn calculate_page_range(
    byte_start: usize,
    byte_end: usize,
    boundaries: &[PageBoundary],
) -> Result<(Option<u32>, Option<u32>)> {
    Ok(PageIndex::new(boundaries)?.range(byte_start, byte_end))
}

/// Calculate the ordered list of pages a byte range spans, as [`PageSpan`]s.
///
/// # Errors
///
/// Returns `XbergError::Validation` if boundaries are invalid.
#[cfg(test)]
pub(crate) fn calculate_page_spans(
    byte_start: usize,
    byte_end: usize,
    boundaries: &[PageBoundary],
) -> Result<Vec<PageSpan>> {
    Ok(PageIndex::new(boundaries)?.spans(byte_start, byte_end))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_page_boundaries_valid() {
        let boundaries = vec![
            PageBoundary {
                byte_start: 0,
                byte_end: 20,
                page_number: 1,
            },
            PageBoundary {
                byte_start: 20,
                byte_end: 40,
                page_number: 2,
            },
            PageBoundary {
                byte_start: 40,
                byte_end: 60,
                page_number: 3,
            },
        ];

        let result = validate_page_boundaries(&boundaries);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_page_boundaries_empty() {
        let boundaries: Vec<PageBoundary> = vec![];
        let result = validate_page_boundaries(&boundaries);
        assert!(result.is_ok());
    }

    #[test]
    fn test_calculate_page_range_within_page() {
        let boundaries = vec![
            PageBoundary {
                byte_start: 0,
                byte_end: 100,
                page_number: 1,
            },
            PageBoundary {
                byte_start: 100,
                byte_end: 200,
                page_number: 2,
            },
        ];

        let (first, last) = calculate_page_range(10, 50, &boundaries).unwrap();
        assert_eq!(first, Some(1));
        assert_eq!(last, Some(1));
    }

    #[test]
    fn test_calculate_page_range_spanning_pages() {
        let boundaries = vec![
            PageBoundary {
                byte_start: 0,
                byte_end: 100,
                page_number: 1,
            },
            PageBoundary {
                byte_start: 100,
                byte_end: 200,
                page_number: 2,
            },
        ];

        let (first, last) = calculate_page_range(50, 150, &boundaries).unwrap();
        assert_eq!(first, Some(1));
        assert_eq!(last, Some(2));
    }

    #[test]
    fn test_calculate_page_range_empty_boundaries() {
        let boundaries: Vec<PageBoundary> = vec![];

        let (first, last) = calculate_page_range(0, 50, &boundaries).unwrap();
        assert_eq!(first, None);
        assert_eq!(last, None);
    }

    #[test]
    fn test_calculate_page_range_no_overlap() {
        let boundaries = vec![
            PageBoundary {
                byte_start: 0,
                byte_end: 100,
                page_number: 1,
            },
            PageBoundary {
                byte_start: 100,
                byte_end: 200,
                page_number: 2,
            },
        ];

        let (first, last) = calculate_page_range(200, 250, &boundaries).unwrap();
        assert_eq!(first, None);
        assert_eq!(last, None);
    }

    #[test]
    fn test_calculate_page_range_three_pages() {
        let boundaries = vec![
            PageBoundary {
                byte_start: 0,
                byte_end: 100,
                page_number: 1,
            },
            PageBoundary {
                byte_start: 100,
                byte_end: 200,
                page_number: 2,
            },
            PageBoundary {
                byte_start: 200,
                byte_end: 300,
                page_number: 3,
            },
        ];

        let (first, last) = calculate_page_range(50, 250, &boundaries).unwrap();
        assert_eq!(first, Some(1));
        assert_eq!(last, Some(3));
    }

    #[test]
    fn test_calculate_page_range_with_invalid_boundaries() {
        let boundaries = vec![PageBoundary {
            byte_start: 15,
            byte_end: 10,
            page_number: 1,
        }];

        let result = calculate_page_range(0, 20, &boundaries);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Invalid boundary range"));
    }

    #[test]
    fn test_page_boundaries_with_gaps() {
        let boundaries = vec![
            PageBoundary {
                byte_start: 0,
                byte_end: 10,
                page_number: 1,
            },
            PageBoundary {
                byte_start: 15,
                byte_end: 25,
                page_number: 2,
            },
        ];

        let result = validate_page_boundaries(&boundaries);
        assert!(result.is_ok());
    }

    #[test]
    fn test_chunk_with_same_start_and_end() {
        let boundaries = vec![PageBoundary {
            byte_start: 10,
            byte_end: 10,
            page_number: 1,
        }];

        let result = validate_page_boundaries(&boundaries);
        assert!(result.is_ok());
    }

    #[test]
    fn test_zero_length_boundary_is_valid() {
        let boundaries = vec![
            PageBoundary {
                byte_start: 0,
                byte_end: 100,
                page_number: 1,
            },
            PageBoundary {
                byte_start: 100,
                byte_end: 100,
                page_number: 2,
            },
            PageBoundary {
                byte_start: 100,
                byte_end: 200,
                page_number: 3,
            },
        ];
        assert!(validate_page_boundaries(&boundaries).is_ok());
    }

    #[test]
    fn should_return_single_page_span_when_chunk_is_within_one_page() {
        let boundaries = vec![
            PageBoundary {
                byte_start: 0,
                byte_end: 100,
                page_number: 1,
            },
            PageBoundary {
                byte_start: 100,
                byte_end: 200,
                page_number: 2,
            },
        ];

        let spans = calculate_page_spans(10, 50, &boundaries).unwrap();
        assert_eq!(spans.len(), 1, "single-page chunk must produce exactly one span");
        assert_eq!(spans[0].page, 1);
        assert_eq!(
            spans[0].bbox, None,
            "bbox is filled in a later pass, not by calculate_page_spans"
        );
    }

    #[test]
    fn should_return_one_span_per_page_when_chunk_spans_multiple_pages() {
        let boundaries = vec![
            PageBoundary {
                byte_start: 0,
                byte_end: 100,
                page_number: 3,
            },
            PageBoundary {
                byte_start: 100,
                byte_end: 200,
                page_number: 4,
            },
            PageBoundary {
                byte_start: 200,
                byte_end: 300,
                page_number: 5,
            },
        ];

        let spans = calculate_page_spans(50, 250, &boundaries).unwrap();
        let pages: Vec<u32> = spans.iter().map(|s| s.page).collect();
        assert_eq!(
            pages,
            vec![3, 4, 5],
            "chunk spanning pages 3-5 must produce one span per page in order"
        );
        assert!(spans.iter().all(|s| s.bbox.is_none()));
    }

    #[test]
    fn should_return_empty_spans_when_boundaries_are_empty() {
        let spans = calculate_page_spans(0, 50, &[]).unwrap();
        assert!(spans.is_empty());
    }

    #[test]
    fn should_return_empty_spans_when_chunk_does_not_overlap_any_page() {
        let boundaries = vec![PageBoundary {
            byte_start: 0,
            byte_end: 100,
            page_number: 1,
        }];

        let spans = calculate_page_spans(200, 250, &boundaries).unwrap();
        assert!(spans.is_empty());
    }

    #[test]
    fn should_match_calculate_page_range_first_and_last_page() {
        let boundaries = vec![
            PageBoundary {
                byte_start: 0,
                byte_end: 100,
                page_number: 1,
            },
            PageBoundary {
                byte_start: 100,
                byte_end: 200,
                page_number: 2,
            },
            PageBoundary {
                byte_start: 200,
                byte_end: 300,
                page_number: 3,
            },
        ];

        let (first_page, last_page) = calculate_page_range(50, 250, &boundaries).unwrap();
        let spans = calculate_page_spans(50, 250, &boundaries).unwrap();

        assert_eq!(spans.first().map(|s| s.page), first_page);
        assert_eq!(spans.last().map(|s| s.page), last_page);
    }

    /// The pre-index algorithms: validate and scan every boundary for every query.
    fn legacy_range(start: usize, end: usize, boundaries: &[PageBoundary]) -> (Option<u32>, Option<u32>, Vec<u32>) {
        let hit: Vec<u32> = boundaries
            .iter()
            .filter(|b| start < b.byte_end && end > b.byte_start)
            .map(|b| b.page_number)
            .collect();
        (hit.first().copied(), hit.last().copied(), hit)
    }

    fn legacy_page_start(offset: usize, boundaries: &[PageBoundary]) -> Option<usize> {
        boundaries
            .iter()
            .find(|b| offset >= b.byte_start && offset < b.byte_end)
            .or_else(|| boundaries.iter().rev().find(|b| b.byte_start <= offset))
            .map(|b| b.byte_start)
    }

    /// Deterministic boundary sets with gaps, zero-width pages and adjacent pages.
    fn boundary_sets() -> Vec<Vec<PageBoundary>> {
        let mut state = 0x2085_u64;
        let mut next = move |bound: usize| {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((state >> 33) as usize) % bound
        };
        (0..40)
            .map(|_| {
                let mut cursor = next(5);
                (1..=next(30) as u32 + 1)
                    .map(|page_number| {
                        cursor += next(4);
                        let byte_start = cursor;
                        cursor += next(9);
                        PageBoundary {
                            byte_start,
                            byte_end: cursor,
                            page_number,
                        }
                    })
                    .collect()
            })
            .collect()
    }

    #[test]
    fn page_index_matches_linear_scan_for_every_range() {
        for boundaries in boundary_sets() {
            let index = PageIndex::new(&boundaries).unwrap();
            let limit = boundaries.last().map_or(0, |b| b.byte_end) + 3;
            for start in 0..=limit {
                assert_eq!(
                    index.page_start(start),
                    legacy_page_start(start, &boundaries),
                    "page_start({start}) over {boundaries:?}"
                );
                for end in start..=limit {
                    let (first, last, pages) = legacy_range(start, end, &boundaries);
                    assert_eq!(
                        index.range(start, end),
                        (first, last),
                        "range({start},{end}) over {boundaries:?}"
                    );
                    let spans: Vec<u32> = index.spans(start, end).iter().map(|s| s.page).collect();
                    assert_eq!(spans, pages, "spans({start},{end}) over {boundaries:?}");
                }
            }
        }
    }

    #[test]
    fn page_index_rejects_invalid_boundaries_at_construction() {
        let unsorted = [
            PageBoundary {
                byte_start: 10,
                byte_end: 20,
                page_number: 1,
            },
            PageBoundary {
                byte_start: 0,
                byte_end: 5,
                page_number: 2,
            },
        ];
        assert!(PageIndex::new(&unsorted).is_err());
    }
}
