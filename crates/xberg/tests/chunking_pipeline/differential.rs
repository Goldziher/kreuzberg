//! Cross-build differential: digests of many adversarial inputs, to diff two implementations.
//!
//! Run in each tree with `XBERG_CHUNK_FUZZ_DUMP=/path`, then compare the files byte for byte.

use std::fmt::Write as _;

use xberg::chunking::ChunkerType;

use crate::corpus;
use crate::digest::{config, digest, run};

#[test]
#[ignore = "diagnostic dump for cross-build comparison"]
fn dump_fuzz_digests() {
    let Some(path) = std::env::var_os("XBERG_CHUNK_FUZZ_DUMP") else {
        return;
    };
    let cases: u64 = std::env::var("XBERG_CHUNK_FUZZ_CASES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(400);
    let mut out = String::new();
    let seed: u64 = std::env::var("XBERG_CHUNK_FUZZ_SEED")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0xd1ff);
    let mut rng = corpus::Rng::new(seed);
    for case in 0..cases {
        let size = rng.range(300, 60_000);
        let text = corpus::fuzz_document(size, rng.next_u64());
        let max = *rng.pick(&[5usize, 12, 40, 100, 333, 1_000, 3_000]);
        let overlap = if rng.below(2) == 0 { 0 } else { rng.below(max.min(200)) };
        let trim = rng.below(4) != 0;
        for (name, chunker) in [
            ("text", ChunkerType::Text),
            ("markdown", ChunkerType::Markdown),
            ("semantic", ChunkerType::Semantic),
        ] {
            let mut cfg = config(chunker, max, overlap);
            cfg.trim = trim;
            let hash = digest(&run(&text, &cfg, None));
            writeln!(
                out,
                "{case} {name} size={size} max={max} overlap={overlap} trim={trim} {hash:016x}"
            )
            .unwrap();
        }
    }
    std::fs::write(path, out).unwrap();
}

/// Writes every chunk of one fuzz case, for locating the first divergence between two builds.
/// `XBERG_CHUNK_FUZZ_CASE=<n> XBERG_CHUNK_FUZZ_CHUNKS=/path`.
#[test]
#[ignore = "diagnostic dump for cross-build comparison"]
fn dump_fuzz_case_chunks() {
    let (Some(path), Some(target)) = (
        std::env::var_os("XBERG_CHUNK_FUZZ_CHUNKS"),
        std::env::var("XBERG_CHUNK_FUZZ_CASE")
            .ok()
            .and_then(|v| v.parse::<u64>().ok()),
    ) else {
        return;
    };
    let mut rng = corpus::Rng::new(0xd1ff);
    for case in 0..=target {
        let size = rng.range(300, 60_000);
        let text = corpus::fuzz_document(size, rng.next_u64());
        let max = *rng.pick(&[5usize, 12, 40, 100, 333, 1_000, 3_000]);
        let overlap = if rng.below(2) == 0 { 0 } else { rng.below(max.min(200)) };
        let trim = rng.below(4) != 0;
        if case != target {
            continue;
        }
        let mut cfg = config(ChunkerType::Text, max, overlap);
        cfg.trim = trim;
        let result = run(&text, &cfg, None);
        let mut out = String::new();
        for chunk in &result.chunks {
            writeln!(
                out,
                "{}..{} {:?}",
                chunk.metadata.byte_start, chunk.metadata.byte_end, chunk.content
            )
            .unwrap();
        }
        std::fs::write(&path, out).unwrap();
    }
}
