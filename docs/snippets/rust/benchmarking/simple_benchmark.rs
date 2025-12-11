```rust
use kreuzberg::{Kreuzberg, ExtractionConfig, CacheConfig};
use std::time::Instant;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = ExtractionConfig {
        use_cache: false,
        ..Default::default()
    };
    let kreuzberg = Kreuzberg::new(config)?;
    let file_path = "document.pdf";
    let num_runs = 10;

    let start = Instant::now();
    for _ in 0..num_runs {
        let _ = kreuzberg.extract_file(file_path).await?;
    }
    let sync_duration = start.elapsed().as_secs_f64();
    let avg_sync = sync_duration / num_runs as f64;

    println!("Sync extraction ({} runs):", num_runs);
    println!("  - Total time: {:.3}s", sync_duration);
    println!("  - Average: {:.3}s per extraction", avg_sync);

    let start = Instant::now();
    let mut tasks = vec![];
    for _ in 0..num_runs {
        tasks.push(kreuzberg.extract_file(file_path));
    }
    let results = futures::future::join_all(tasks).await;
    for result in results {
        result?;
    }
    let async_duration = start.elapsed().as_secs_f64();

    println!("\nAsync extraction ({} parallel runs):", num_runs);
    println!("  - Total time: {:.3}s", async_duration);
    println!("  - Average: {:.3}s per extraction", async_duration / num_runs as f64);
    println!("  - Speedup: {:.1}x", sync_duration / async_duration);

    let cache_config = ExtractionConfig {
        use_cache: true,
        ..Default::default()
    };
    let kreuzberg_cached = Kreuzberg::new(cache_config)?;

    println!("\nFirst extraction (populates cache)...");
    let start = Instant::now();
    let result1 = kreuzberg_cached.extract_file(file_path).await?;
    let first_duration = start.elapsed().as_secs_f64();
    println!("  - Time: {:.3}s", first_duration);

    println!("Second extraction (from cache)...");
    let start = Instant::now();
    let result2 = kreuzberg_cached.extract_file(file_path).await?;
    let cached_duration = start.elapsed().as_secs_f64();
    println!("  - Time: {:.3}s", cached_duration);
    println!("  - Cache speedup: {:.1}x", first_duration / cached_duration);

    Ok(())
}
```
