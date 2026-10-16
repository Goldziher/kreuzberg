//! End-to-end tests for `split_and_extract` (multi-document PDF splitting).
//!
//! Fixture required (relative to `test_documents/`):
//! - `pdf/multi_page.pdf` — any native-text PDF with ≥ 2 pages.
//!
//! Like the other PDF integration suites, these skip when the fixture is absent
//! so local runs without the corpus stay green; CI provides the fixture.

#![allow(clippy::print_stdout, clippy::print_stderr, clippy::dbg_macro)] // ~keep: test/bench binaries print by design; org logging policy exempts tests
#![cfg(all(feature = "pdf", feature = "heuristics"))]

mod helpers;
use helpers::{get_test_file_path, skip_if_missing};

use xberg::{SplitConfig, SplitStrategy, split_and_extract};

/// `Auto` on a cohesive multi-page PDF must return contiguous segments that
/// cover every page exactly once, each carrying its own extracted content.
#[tokio::test]
async fn auto_split_covers_all_pages_contiguously() {
    if skip_if_missing("pdf/multi_page.pdf") {
        eprintln!("skipping: fixture pdf/multi_page.pdf not found");
        return;
    }
    let bytes = std::fs::read(get_test_file_path("pdf/multi_page.pdf")).expect("read multi_page.pdf");

    let segments = split_and_extract(&bytes, &SplitConfig::default())
        .await
        .expect("auto split_and_extract should succeed");

    assert!(!segments.is_empty(), "auto split must produce at least one segment");

    let mut expected_start = 1u32;
    for segment in &segments {
        assert_eq!(
            *segment.page_range.start(),
            expected_start,
            "segments must be contiguous"
        );
        let span = segment.page_range.end() - segment.page_range.start() + 1;
        assert_eq!(
            segment.document.counts.pages as u32, span,
            "segment counts.pages must match its page-range span"
        );
        expected_start = segment.page_range.end() + 1;
    }

    let total_pages = expected_start - 1;
    assert!(total_pages >= 2, "fixture must have ≥ 2 pages");
}

/// Caller-supplied `PageRanges` must slice exactly the requested pages from the
/// single parse.
#[tokio::test]
async fn page_ranges_split_slices_requested_pages() {
    if skip_if_missing("pdf/multi_page.pdf") {
        eprintln!("skipping: fixture pdf/multi_page.pdf not found");
        return;
    }
    let bytes = std::fs::read(get_test_file_path("pdf/multi_page.pdf")).expect("read multi_page.pdf");

    let full = split_and_extract(&bytes, &SplitConfig::default())
        .await
        .expect("auto split to learn page count");
    let total_pages = full.last().map(|s| *s.page_range.end()).expect("at least one segment");
    assert!(total_pages >= 2, "fixture must have ≥ 2 pages");

    let config = SplitConfig {
        strategy: SplitStrategy::PageRanges(vec![1..=1, 2..=total_pages]),
        ..Default::default()
    };
    let segments = split_and_extract(&bytes, &config)
        .await
        .expect("page-ranges split_and_extract should succeed");

    assert_eq!(segments.len(), 2, "two requested ranges → two segments");
    assert_eq!(segments[0].page_range, 1..=1);
    assert_eq!(segments[0].document.counts.pages, 1);
    assert_eq!(segments[1].page_range, 2..=total_pages);
    assert_eq!(segments[1].document.counts.pages as u32, total_pages - 1);
}

/// An out-of-bounds page range is rejected before any partitioning.
#[tokio::test]
async fn page_ranges_out_of_bounds_is_rejected() {
    if skip_if_missing("pdf/multi_page.pdf") {
        eprintln!("skipping: fixture pdf/multi_page.pdf not found");
        return;
    }
    let bytes = std::fs::read(get_test_file_path("pdf/multi_page.pdf")).expect("read multi_page.pdf");

    let config = SplitConfig {
        strategy: SplitStrategy::PageRanges(std::iter::once(1..=9999).collect()),
        ..Default::default()
    };
    let result = split_and_extract(&bytes, &config).await;
    assert!(result.is_err(), "range beyond the page count must error");
}
