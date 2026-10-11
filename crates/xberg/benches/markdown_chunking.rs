//! Tracks Markdown splitter scaling for text-like inputs with many parser ranges. ~keep

use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use xberg::chunking::{ChunkerType, ChunkingConfig, chunk_text};

fn repeated_to_size(pattern: &str, size: usize) -> String {
    let mut text = pattern.repeat(size.div_ceil(pattern.len()));
    text.truncate(size);
    text
}

fn bench_markdown_chunking(criterion: &mut Criterion) {
    let config = ChunkingConfig {
        max_characters: 1_000,
        overlap: 200,
        trim: true,
        chunker_type: ChunkerType::Markdown,
        ..Default::default()
    };
    let mut group = criterion.benchmark_group("markdown_chunking");
    group.sample_size(10);

    for size in [64 * 1_024, 128 * 1_024, 256 * 1_024, 1_024 * 1_024] {
        for (profile, pattern) in [
            ("log_like", "2021-01-27 14:00:00 host1 svc[1]: event id=1 status=ok\n"),
            ("dense_inline", "*alpha* `beta` [gamma](delta) "),
        ] {
            let text = repeated_to_size(pattern, size);
            group.throughput(Throughput::Bytes(size as u64));
            group.bench_with_input(BenchmarkId::new(profile, size), &text, |bencher, text| {
                bencher.iter(|| chunk_text(black_box(text), black_box(&config), None).unwrap());
            });
        }
    }

    group.finish();
}

criterion_group!(benches, bench_markdown_chunking);
criterion_main!(benches);
