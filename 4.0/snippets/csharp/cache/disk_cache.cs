```csharp
using Kreuzberg;
using System;
using System.IO;
using System.Threading.Tasks;

// Configure disk cache
var config = new ExtractionConfig
{
    UseCache = true,
    CacheConfig = new CacheConfig
    {
        CachePath = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "kreuzberg_cache"),
        MaxCacheSize = 1024 * 1024 * 500, // 500 MB
        CacheTtlSeconds = 86400 * 7,      // 7 days
        EnableCompression = true
    }
};

// First extraction - will be cached
Console.WriteLine("First extraction (will be cached)...");
var result1 = await KreuzbergClient.ExtractFileAsync("document.pdf", config);
Console.WriteLine($"  - Content length: {result1.Content.Length}");
Console.WriteLine($"  - Cached: {result1.Metadata.WasCached}");

// Second extraction - will use cache
Console.WriteLine("\nSecond extraction (from cache)...");
var result2 = await KreuzbergClient.ExtractFileAsync("document.pdf", config);
Console.WriteLine($"  - Content length: {result2.Content.Length}");
Console.WriteLine($"  - Cached: {result2.Metadata.WasCached}");

// Both results should be identical
Console.WriteLine($"\nResults are identical: {result1.Content == result2.Content}");

// Clear cache for specific file
await KreuzbergClient.ClearCacheAsync("document.pdf");
Console.WriteLine("\nCache cleared for document.pdf");

// Clear all cache
await KreuzbergClient.ClearAllCacheAsync();
Console.WriteLine("All cache cleared");

// Get cache statistics
var cacheStats = await KreuzbergClient.GetCacheStatsAsync();
Console.WriteLine($"\nCache Statistics:");
Console.WriteLine($"  - Total entries: {cacheStats.TotalEntries}");
Console.WriteLine($"  - Cache size: {cacheStats.CacheSizeBytes / 1024 / 1024} MB");
Console.WriteLine($"  - Hit rate: {cacheStats.HitRate:P}");
```
