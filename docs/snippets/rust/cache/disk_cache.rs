```rust
use kreuzberg::{Kreuzberg, ExtractionConfig, CacheConfig};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cache_dir = dirs::cache_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine cache directory"))?
        .join("kreuzberg");
    std::fs::create_dir_all(&cache_dir)?;

    let config = ExtractionConfig {
        use_cache: true,
        cache_config: Some(CacheConfig {
            cache_path: cache_dir,
            max_cache_size: 500 * 1024 * 1024,
            cache_ttl_seconds: 7 * 86400,
            enable_compression: true,
        }),
        ..Default::default()
    };

    let kreuzberg = Kreuzberg::new(config)?;

    println!("First extraction (will be cached)...");
    let result1 = kreuzberg.extract_file("document.pdf").await?;
    println!("  - Content length: {}", result1.content.len());
    println!("  - Cached: {}", result1.metadata.was_cached);

    println!("\nSecond extraction (from cache)...");
    let result2 = kreuzberg.extract_file("document.pdf").await?;
    println!("  - Content length: {}", result2.content.len());
    println!("  - Cached: {}", result2.metadata.was_cached);

    println!("\nResults are identical: {}", result1.content == result2.content);

    let cache_stats = kreuzberg.get_cache_stats().await?;
    println!("\nCache Statistics:");
    println!("  - Total entries: {}", cache_stats.total_entries);
    println!("  - Cache size: {:.1} MB", cache_stats.cache_size_bytes as f64 / 1024.0 / 1024.0);
    println!("  - Hit rate: {:.1}%", cache_stats.hit_rate * 100.0);

    Ok(())
}
```
