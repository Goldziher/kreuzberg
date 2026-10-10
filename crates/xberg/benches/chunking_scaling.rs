//! Chunking cost at 1x/2x/4x/8x of a base size for every document shape and chunker type, so a
//! super-linear regression shows as a growing time-per-byte rather than a flat one (GH#2085).
//! Shapes come from the same generators the `chunking_pipeline` tests use. ~keep

#[path = "../tests/chunking_pipeline/corpus.rs"]
mod corpus;

use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use xberg::chunking::{ChunkerType, ChunkingConfig, chunk_text};

const BASE_BYTES: usize = 128 * 1_024;
const FACTORS: [usize; 4] = [1, 2, 4, 8];

fn config(chunker_type: ChunkerType) -> ChunkingConfig {
    ChunkingConfig {
        max_characters: 1_000,
        overlap: 200,
        trim: true,
        chunker_type,
        ..Default::default()
    }
}

fn bench_chunking_scaling(criterion: &mut Criterion) {
    for (chunker_name, chunker_type) in [
        ("markdown", ChunkerType::Markdown),
        ("text", ChunkerType::Text),
        ("semantic", ChunkerType::Semantic),
    ] {
        let cfg = config(chunker_type);
        let mut group = criterion.benchmark_group(format!("chunking_scaling/{chunker_name}"));
        group.sample_size(10);
        for (shape, generator) in corpus::SHAPES {
            for factor in FACTORS {
                let size = BASE_BYTES * factor;
                let text = generator(size, 7);
                group.throughput(Throughput::Bytes(text.len() as u64));
                group.bench_with_input(BenchmarkId::new(*shape, factor), &text, |bencher, text| {
                    bencher.iter(|| chunk_text(black_box(text), black_box(&cfg), None).unwrap());
                });
            }
        }
        group.finish();
    }
}

fn bench_paged_chunking_scaling(criterion: &mut Criterion) {
    let cfg = config(ChunkerType::Markdown);
    let mut group = criterion.benchmark_group("chunking_scaling/paged_markdown");
    group.sample_size(10);
    for factor in FACTORS {
        let text = corpus::flat_prose(BASE_BYTES * factor, 7);
        let pages = corpus::pages(&text, 256);
        group.throughput(Throughput::Bytes(text.len() as u64));
        group.bench_with_input(BenchmarkId::new("flat_prose", factor), &text, |bencher, text| {
            bencher.iter(|| chunk_text(black_box(text), black_box(&cfg), Some(black_box(&pages))).unwrap());
        });
    }
    group.finish();
}

criterion_group!(benches, bench_chunking_scaling, bench_paged_chunking_scaling);
criterion_main!(benches);
