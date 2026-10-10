//! Output equivalence with the splitter before the scaling fixes (GH#2085).
//!
//! `golden.txt` was produced by the pre-optimization implementation over deterministic generated
//! documents. Every digest covers chunk text, offsets, page range and spans, heading context and
//! classification, so any behavioural drift in the optimized paths fails here.
//!
//! Behaviour changes since that baseline, each regenerating only the lines it touches:
//! exact semantic offsets (semantic), source ranges for YAML/JSON sections (yaml), and the
//! content-free-chunk fix (text, markdown, semantic). Everything else is byte-for-byte what the
//! pre-optimization code produced. Regenerate only for an intentional output change:
//! `XBERG_CHUNK_GOLDEN_WRITE=1 cargo test --test chunking_pipeline golden -- --nocapture`.

use std::fmt::Write as _;

use crate::corpus::{self, SHAPES};
use crate::digest::{ALL_CHUNKERS, CAPACITIES, config, digest, run};
use crate::fixtures;

const GOLDEN: &str = include_str!("golden.txt");
const SIZE: usize = 48 * 1024;

fn generated_lines() -> String {
    let mut out = String::new();
    for (name, generator) in SHAPES {
        let text = generator(SIZE, 2085);
        let contiguous = corpus::pages(&text, 1_300);
        let gapped = fixtures::gapped_pages(&text, 900);
        for (chunker_name, chunker) in ALL_CHUNKERS {
            for &(max, overlap) in CAPACITIES {
                let cfg = config(*chunker, max, overlap);
                for (pages_name, pages) in [
                    ("none", None),
                    ("contiguous", Some(&contiguous)),
                    ("gapped", Some(&gapped)),
                ] {
                    let hash = digest(&run(&text, &cfg, pages.map(Vec::as_slice)));
                    writeln!(out, "{name} {chunker_name} {max} {overlap} {pages_name} {hash:016x}").unwrap();
                }
            }
        }
    }
    out
}

#[test]
fn generated_corpus_output_matches_pre_optimization_golden() {
    let actual = generated_lines();
    if std::env::var_os("XBERG_CHUNK_GOLDEN_WRITE").is_some() {
        std::fs::write(
            concat!(env!("CARGO_MANIFEST_DIR"), "/tests/chunking_pipeline/golden.txt"),
            &actual,
        )
        .unwrap();
        return;
    }
    let mismatches: Vec<String> = actual
        .lines()
        .zip(GOLDEN.lines())
        .filter(|(a, g)| a != g)
        .map(|(a, g)| format!("  now    {a}\n  golden {g}"))
        .collect();
    assert_eq!(
        actual.lines().count(),
        GOLDEN.lines().count(),
        "golden case count changed"
    );
    assert!(
        mismatches.is_empty(),
        "chunk output drifted from golden:\n{}",
        mismatches.join("\n")
    );
}

/// Dumps digests of the real-document corpus so two builds can be diffed with `diff`.
/// `XBERG_CHUNK_REAL_DUMP=/path cargo test --test chunking_pipeline dump_real -- --ignored`.
#[test]
#[ignore = "diagnostic dump for cross-build comparison"]
fn dump_real_document_digests() {
    let Some(path) = std::env::var_os("XBERG_CHUNK_REAL_DUMP") else {
        return;
    };
    let mut out = String::new();
    for (name, text) in fixtures::real_documents() {
        let pages = corpus::pages(&text, 1_500);
        for (chunker_name, chunker) in ALL_CHUNKERS {
            for &(max, overlap) in CAPACITIES {
                let cfg = config(*chunker, max, overlap);
                for (pages_name, p) in [("none", None), ("pages", Some(&pages))] {
                    let hash = digest(&run(&text, &cfg, p.map(Vec::as_slice)));
                    writeln!(out, "{name} {chunker_name} {max} {overlap} {pages_name} {hash:016x}").unwrap();
                }
            }
        }
    }
    std::fs::write(path, out).unwrap();
}

/// Dumps every chunk's byte range for the real and generated corpus, to count boundary changes
/// between two builds. `XBERG_CHUNK_CUTS_DUMP=/path`.
#[test]
#[ignore = "diagnostic dump for cross-build comparison"]
fn dump_chunk_ranges() {
    let Some(path) = std::env::var_os("XBERG_CHUNK_CUTS_DUMP") else {
        return;
    };
    let mut docs = fixtures::real_documents();
    for (name, generator) in SHAPES {
        docs.push((format!("gen/{name}"), generator(SIZE, 2085)));
    }
    let mut out = String::new();
    for (name, text) in docs {
        for (chunker_name, chunker) in crate::digest::CHUNKERS {
            for &(max, overlap) in CAPACITIES {
                let result = run(&text, &config(*chunker, max, overlap), None);
                let ranges: Vec<String> = result
                    .chunks
                    .iter()
                    .map(|c| format!("{}:{}", c.metadata.byte_start, c.metadata.byte_end))
                    .collect();
                writeln!(out, "{name} {chunker_name} {max} {overlap} {}", ranges.join(",")).unwrap();
            }
        }
    }
    std::fs::write(path, out).unwrap();
}
