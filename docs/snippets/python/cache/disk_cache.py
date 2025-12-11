```python
from pathlib import Path
from kreuzberg import Kreuzberg, ExtractionConfig, CacheConfig

cache_dir = Path.home() / ".cache" / "kreuzberg"
cache_dir.mkdir(parents=True, exist_ok=True)

config = ExtractionConfig(
    use_cache=True,
    cache_config=CacheConfig(
        cache_path=str(cache_dir),
        max_cache_size=500 * 1024 * 1024,
        cache_ttl_seconds=7 * 86400,
        enable_compression=True,
    ),
)

kreuzberg = Kreuzberg(config)

print("First extraction (will be cached)...")
result1 = kreuzberg.extract_file("document.pdf")
print(f"  - Content length: {len(result1.content)}")
print(f"  - Cached: {result1.metadata.get('was_cached', False)}")

print("\nSecond extraction (from cache)...")
result2 = kreuzberg.extract_file("document.pdf")
print(f"  - Content length: {len(result2.content)}")
print(f"  - Cached: {result2.metadata.get('was_cached', False)}")

print(f"\nResults are identical: {result1.content == result2.content}")

cache_stats = kreuzberg.get_cache_stats()
print(f"\nCache Statistics:")
print(f"  - Total entries: {cache_stats.get('total_entries', 0)}")
print(f"  - Cache size: {cache_stats.get('cache_size_bytes', 0) / 1024 / 1024:.1f} MB")
print(f"  - Hit rate: {cache_stats.get('hit_rate', 0):.1%}")
```
