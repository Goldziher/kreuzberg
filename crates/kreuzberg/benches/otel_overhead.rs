use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_text_extraction(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("extract_text_no_otel", |b| {
        b.iter(|| {
            runtime.block_on(async {
                use kreuzberg::core::config::ExtractionConfig;
                use kreuzberg::core::extractor::extract_bytes;

                let test_content = black_box(b"Hello, World! This is a test document.");
                let config = ExtractionConfig::default();

                extract_bytes(test_content, "text/plain", &config).await
            })
        });
    });
}

fn bench_cache_operations(c: &mut Criterion) {
    use kreuzberg::cache::GenericCache;
    use tempfile::tempdir;

    let temp_dir = tempdir().unwrap();
    let cache = GenericCache::new(
        "bench".to_string(),
        Some(temp_dir.path().to_str().unwrap().to_string()),
        30.0,
        500.0,
        1000.0,
    )
    .unwrap();

    c.bench_function("cache_set_get", |b| {
        b.iter(|| {
            let key = black_box("bench_key");
            let data = black_box(b"benchmark data".to_vec());

            cache.set(key, data.clone(), None).unwrap();
            cache.get(key, None).unwrap()
        });
    });
}

criterion_group!(benches, bench_text_extraction, bench_cache_operations);
criterion_main!(benches);
