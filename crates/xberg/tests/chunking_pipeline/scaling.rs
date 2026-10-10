//! Regression guard against super-linear chunking cost (GH#2085).
//!
//! Each shape is chunked at a base size and at 8x that size. Linear work grows ~8x, quadratic
//! work ~64x, so the guard fails well past honest noise (24x) and well short of a regression.

use std::time::Instant;

use xberg::types::PageBoundary;

use crate::corpus::{self, SHAPES};
use crate::digest::{CHUNKERS, config, run};

const GROWTH: usize = 8;
const MAX_RATIO: f64 = 24.0;
const MIN_BASELINE_MS: f64 = 0.25;

fn base_size() -> usize {
    if cfg!(debug_assertions) { 32 * 1024 } else { 256 * 1024 }
}

fn best_ms(runs: usize, mut f: impl FnMut()) -> f64 {
    (0..runs)
        .map(|_| {
            let start = Instant::now();
            f();
            start.elapsed().as_secs_f64() * 1e3
        })
        .fold(f64::INFINITY, f64::min)
}

fn growth_ratio(make: &dyn Fn(usize) -> (String, Option<Vec<PageBoundary>>), chunker_index: usize) -> f64 {
    let (_, chunker) = CHUNKERS[chunker_index];
    let cfg = config(chunker, 1_000, 200);
    let measure = |size: usize| {
        let (text, pages) = make(size);
        best_ms(3, || {
            std::hint::black_box(run(&text, &cfg, pages.as_deref()));
        })
    };
    measure(base_size() * GROWTH) / measure(base_size()).max(MIN_BASELINE_MS)
}

fn assert_linear(label: &str, make: &dyn Fn(usize) -> (String, Option<Vec<PageBoundary>>), chunker_index: usize) {
    let ratio = growth_ratio(make, chunker_index).min(growth_ratio(make, chunker_index));
    assert!(
        ratio <= MAX_RATIO,
        "{label}: {GROWTH}x input took {ratio:.1}x as long (linear ~{GROWTH}x, quadratic ~{}x, limit {MAX_RATIO}x)",
        GROWTH * GROWTH
    );
}

#[test]
fn chunking_cost_grows_linearly_with_document_size() {
    for (name, generator) in SHAPES {
        for (chunker_index, (chunker_name, _)) in CHUNKERS.iter().enumerate() {
            let make = |size: usize| (generator(size, 7), None);
            assert_linear(&format!("{name}/{chunker_name}"), &make, chunker_index);
        }
    }
}

#[test]
fn chunking_cost_grows_linearly_with_page_count() {
    for (chunker_index, (chunker_name, _)) in CHUNKERS.iter().enumerate() {
        let make = |size: usize| {
            let text = corpus::flat_prose(size, 7);
            let pages = corpus::pages(&text, 64);
            (text, Some(pages))
        };
        assert_linear(&format!("paged flat_prose/{chunker_name}"), &make, chunker_index);
    }
}
